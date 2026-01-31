use cu29::prelude::*;
use rmp_serde::to_vec_named;
use serde::Serialize;
use zenoh::{Session, Config, pubsub::Publisher, key_expr::{KeyExpr}, Wait};
use core::marker::PhantomData;

pub struct ZSink<P>
where
    P: CuMsgPayload,
{
    _marker: PhantomData<P>,
    config: ZCfg,
    ctx: Option<ZCtx>,
}

pub struct ZCfg {
    config: Config,
    topic: String,
}

pub struct ZCtx {
    session: Session,
    publisher: Publisher<'static>,
}

impl<P> Freezable for ZSink<P> where P: CuMsgPayload {}

impl<P> CuSinkTask for ZSink<P>
where
    P: CuMsgPayload + 'static + Serialize,
{
    type Input<'m> = input_msg!(P);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let config = config.ok_or(CuError::from("ZSrc: missing config! provide at least no value for the \"topic\" field"))?;

        let session_config = config.get::<String>("zenoh_config_file").map_or(
            Ok(Config::default()),
            |s| -> CuResult<Config> {
                Config::from_file(&s)
                    .map_err(|_| -> CuError {CuError::from("ZSink: Failed to create zenoh config")} )
            },
        )?;

        let topic = config.get::<String>("topic").unwrap_or("palanuk".to_owned());

        Ok(Self {
            _marker: Default::default(),
            config: ZCfg {
                config: session_config,
                topic,
            },
            ctx: None,
        })
    }

    fn start(&mut self, _clock: &RobotClock) -> CuResult<()> {
        let session = Wait::wait(zenoh::open(self.config.config.clone()))
            .map_err(
                |_| -> CuError {CuError::from("ZSink: Failed to open session")}
            )?;

        let key_expr = KeyExpr::<'static>::new(self.config.topic.clone())
            .map_err(
                |_| -> CuError {CuError::from("ZSink: Invalid topic string")}
            )?;

        debug!("Zenoh session open");
        let publisher = Wait::wait(session.declare_publisher(key_expr))
            .map_err(
                |_| -> CuError {CuError::from("ZSink: failed to declare publisher")}
            )?;

        self.ctx = Some(ZCtx { session, publisher });
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, input: &Self::Input<'_>) -> CuResult<()> {
        let ctx = self
            .ctx
            .as_mut()
            .ok_or_else(|| CuError::from("ZSink: Context not found"))?;

        let encoded = match to_vec_named(&input) {
            Ok(ret) => ret,
            Err(_) => return Err(CuError::from(format!("failed to encode")))
        };

        Wait::wait(ctx.publisher.put(encoded))
            .map_err(
                |_| -> CuError {CuError::from("failed to put sample")}
            )?;

        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        if let Some(ZCtx { session, publisher }) = self.ctx.take() {
            Wait::wait(publisher.undeclare())
                .map_err(
                    |_| -> CuError {CuError::from("ZSink: Failed to undeclare publisher")}
                )?;

            Wait::wait(session.close())
                .map_err(
                    |_| -> CuError {CuError::from("ZSink: Failed to close session")}
                )?;
        }
        Ok(())
    }
}
