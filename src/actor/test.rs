#[test]
fn test() {
    use std::time::Duration;

    use tokio::task::yield_now;
    use tokio::time::sleep;

    use crate::actor::unit::utils::stop_unit;
    use crate::{unit::UnitEntry, util::loader::load_units_from_dir};

    use super::{
        unit::utils::{start_unit, update_units},
        Actors,
    };

    async fn wait() {
        sleep(Duration::from_secs_f64(0.01)).await
    }
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let actors = Actors::new();
            update_units(&actors.store, load_units_from_dir("./units").await).await;

            // start unit with its dep
            start_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;

            // stop the active unit
            stop_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;

            // start a stopped unit, with its dep started
            start_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;
            stop_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;

            // start the stopping unit
            start_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;
            stop_unit(&actors.store, UnitEntry::from("t0.service")).await;
            yield_now().await;
            start_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;
            stop_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;

            // stop a starting unit
            start_unit(&actors.store, UnitEntry::from("t0.service")).await;
            stop_unit(&actors.store, UnitEntry::from("t0.service")).await;
            wait().await;

            stop_unit(&actors.store, UnitEntry::from("t1.service")).await;
            wait().await;
        });
}
