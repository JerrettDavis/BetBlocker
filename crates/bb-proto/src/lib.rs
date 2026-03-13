#[allow(clippy::pedantic)]
pub mod device {
    include!(concat!(env!("OUT_DIR"), "/betblocker.device.rs"));
}

#[allow(clippy::pedantic)]
pub mod heartbeat {
    include!(concat!(env!("OUT_DIR"), "/betblocker.heartbeat.rs"));
}

#[allow(clippy::pedantic)]
pub mod blocklist {
    include!(concat!(env!("OUT_DIR"), "/betblocker.blocklist.rs"));
}

#[allow(clippy::pedantic)]
pub mod events {
    include!(concat!(env!("OUT_DIR"), "/betblocker.events.rs"));
}
