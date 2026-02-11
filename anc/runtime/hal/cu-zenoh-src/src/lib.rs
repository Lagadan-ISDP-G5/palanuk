use cu29::prelude::*;
use serde::de::DeserializeOwned;
use rmp_serde::from_slice;
use zenoh::{Config, Session, handlers::{FifoChannel, FifoChannelHandler}, key_expr::KeyExpr, pubsub::Subscriber, sample::Sample};
use core::marker::PhantomData;

pub const CHANNEL_CAPACITY: usize = 2048;
pub const DEFAULT_STALENESS_TIMEOUT_MS: f64 = 500.0;

pub struct ZSrc<S>
where
    S: CuMsgPayload,
{
    _marker: PhantomData<S>,
    config: ZCfg,
    ctx: Option<ZCtx>,
    last_value: Option<S>,
    last_recv_instant: Option<CuInstant>,
    staleness_timeout: CuDuration,
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
    S: CuMsgPayload + 'static + DeserializeOwned + Copy,
{
    type Output<'m> = output_msg!(S);
    type Resources<'r> = ();

    fn new(config: Option<&ComponentConfig>, _resources: Self::Resources<'_>) -> CuResult<Self>
    where
        Self: Sized,
    {
        let config = config.ok_or(CuError::from("ZSrc: missing config! provide at least no value for the \"topic\" field"))?;

        // let router_endpoint = config.get::<String>("router-endpoint").unwrap_or("tcp/localhost:7447".to_owned());

        let def_cfg = Config::default();

        // def_cfg.insert_json5("mode", r#""client""#)
        //     .map_err(|_| -> CuError {CuError::from("ZSrc: Failed to set client mode")})?;

        // def_cfg.insert_json5("connect/endpoints", &format!(r#"["{}"]"#, router_endpoint))
        //     .map_err(|_| -> CuError {CuError::from("ZSrc: Failed to set router endpoint")})?;

        let session_config = config.get::<String>("zenoh_config_file").map_or(
            Ok(def_cfg),
            |s| -> CuResult<Config> {
                Config::from_file(&s)
                    .map_err(|_| -> CuError {CuError::from("ZSrc: Failed to create zenoh config")} )
            },
        )?;

        let topic = config.get::<String>("topic").unwrap_or("palanuk".to_owned());
        let staleness_timeout_ms = config.get::<f64>("staleness_timeout_ms").unwrap_or(DEFAULT_STALENESS_TIMEOUT_MS);

        Ok(Self {
            _marker: Default::default(),
            config: ZCfg {
                config: session_config,
                topic,
            },
            ctx: None,
            last_value: None,
            last_recv_instant: None,
            staleness_timeout: CuDuration::from_millis(staleness_timeout_ms as u64),
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
        let subscriber = zenoh::Wait::wait(session.declare_subscriber(key_expr).with(FifoChannel::new(CHANNEL_CAPACITY)))
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


        match ctx.subscriber.try_recv() {
            Ok(Some(sample)) => {
                let msg = from_slice::<S>(&sample.payload().to_bytes()).map_err(
                    |_| -> CuError {CuError::from("decode failed")}
                )?;
                self.last_value = Some(msg);
                self.last_recv_instant = Some(Instant::now());
            },
            Ok(None) => (), // no new message, will use last_value if not stale
            Err(_) => return Err(CuError::from("msg recv failed"))
        };

        // clear stale values so downstream tasks early-return and motors stop
        if let Some(last_recv) = self.last_recv_instant {
            let res = CuInstant::now().as_nanos().checked_sub(last_recv.as_nanos());
            if let Some(res_nanos) = res {
                let elapsed = CuDuration::from_nanos(res_nanos);
                if elapsed > self.staleness_timeout {
                    self.last_value = None;
                    self.last_recv_instant = None;
                }
            }
        }

        if let Some(ref value) = self.last_value {
            output.set_payload(*value);
        }

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
