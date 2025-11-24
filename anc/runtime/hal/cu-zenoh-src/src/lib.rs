use cu29::prelude::*;
use bincode::decode_from_slice;
use zenoh::{Config, Session, handlers::{FifoChannel, FifoChannelHandler}, key_expr::KeyExpr, pubsub::Subscriber, sample::Sample};
use core::marker::PhantomData;

pub struct ZSrc<S>
where
    S: CuMsgPayload,
{
    _marker: PhantomData<S>,
    config: ZCfg,
    ctx: Option<ZCtx>,
}

pub struct ZCfg {
    config: Config,
    topic: String,
}

pub struct ZCtx {
    session: Session,
    subscriber: Subscriber<FifoChannelHandler<Sample>>,
}

impl<S> Freezable for ZSrc<S> where S: CuMsgPayload {}

impl<S> CuSrcTask for ZSrc<S>
where
    S: CuMsgPayload + 'static,
{
    type Output<'m> = output_msg!(S);

    fn new(config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let config = config.ok_or(CuError::from("ZSrc: missing config! provide at least no value for the \"topic\" field"))?;

        let session_config = config.get::<String>("zenoh_config_file").map_or(
            Ok(Config::default()),
            |s| -> CuResult<Config> {
                Config::from_file(&s)
                    .map_err(|_| -> CuError {CuError::from("ZSrc: Failed to create zenoh config")} )
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
        let session = zenoh::Wait::wait(zenoh::open(self.config.config.clone()))
            .map_err(
                |_| -> CuError {CuError::from("ZSrc: Failed to open session")}
            )?;

        let key_expr = KeyExpr::<'static>::new(self.config.topic.clone())
            .map_err(
                |_| -> CuError {CuError::from("ZSrc: Invalid topic string")}
            )?;

        debug!("Zenoh session open");
        let subscriber = zenoh::Wait::wait(session.declare_subscriber(key_expr).with(FifoChannel::new(2048)))
            .map_err(
                |_| -> CuError {CuError::from("ZSrc: Failed to create subscriber")}
            )?;

        self.ctx = Some(ZCtx { session, subscriber });
        Ok(())
    }

    fn process(&mut self, _clock: &RobotClock, output: &mut Self::Output<'_>) -> CuResult<()> {
        let ctx = self
            .ctx
            .as_mut()
            .ok_or_else(|| CuError::from("ZSrc: Context not found"))?;

        let sample = ctx.subscriber.recv().map_err(
            |_| -> CuError {CuError::from("failed to receive sample")}
        )?;

        let (msg, _) = decode_from_slice(
            sample.payload().to_bytes().trim_ascii(),
            bincode::config::standard())
            .map_err(
                |_| -> CuError {CuError::from("decode failed")}
            )?;

        output.set_payload(msg);
        Ok(())
    }

    fn stop(&mut self, _clock: &RobotClock) -> CuResult<()> {
        if let Some(ZCtx { session, subscriber }) = self.ctx.take() {
            zenoh::Wait::wait(subscriber.undeclare())
                .map_err(
                    |_| -> CuError {CuError::from("ZSrc: Failed to undeclare subscriber")}
                )?;

            zenoh::Wait::wait(session.close())
                .map_err(
                    |_| -> CuError {CuError::from("ZSrc: Failed to close session")}
                )?;
        }
        Ok(())
    }
}
