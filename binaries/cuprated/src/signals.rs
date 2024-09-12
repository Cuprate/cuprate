use tokio::sync::RwLock;

pub static REORG_LOCK: RwLock<()> = RwLock::const_new(());
