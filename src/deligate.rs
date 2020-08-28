pub struct Deligate {}

impl Default for Deligate {
    fn default() -> Self {
        Self {}
    }
}

impl druid::AppDelegate<crate::AppState> for Deligate {}
