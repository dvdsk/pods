use presenter::UserIntent;
use std::time::Duration;
use traits::{DataUpdate, DataUpdateVariant, PodcastId};

#[derive(Debug)]
pub enum AppUpdateVariant {
    Exit,
    Error,
    SearchResults,
}

pub enum Condition<'a> {
    None,
    DataUpdateAndFnMut {
        update: DataUpdateVariant,
        func: Box<dyn FnMut(&DataUpdate) -> bool + Send + 'a>,
    },
    AppUpdate(AppUpdateVariant),
    DataUpdate(DataUpdateVariant),
}

impl std::fmt::Debug for Condition<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Condition::DataUpdateAndFnMut { update, .. } => {
                write!(f, "Condition::FnMut{{ DataUpdate::{update:?}, .. }}")
            }
            Condition::None => write!(f, "Condition::None"),
            Condition::AppUpdate(u) => write!(f, "Condition::AppUpdate({u:?})"),
            Condition::DataUpdate(u) => write!(f, "Condition::DataUpdate({u:?})"),
        }
    }
}

#[derive(Debug)]
pub enum ViewableData {
    PodcastList,
    Episode,
    Podcast { podcast_id: PodcastId },
}

#[derive(Debug)]
pub enum Action {
    View(ViewableData),
    Intent(UserIntent),
    Stop,
}

#[derive(Debug)]
pub struct Steps<'a> {
    pub list: Vec<(Condition<'a>, Action)>,
    pub timeout: Duration,
}

pub struct StepsWNextCondition<'a> {
    next_condition: Condition<'a>,
    list: Vec<(Condition<'a>, Action)>,
    timeout: Duration,
}

impl<'a> StepsWNextCondition<'a> {
    pub fn then_do(mut self, intent: UserIntent) -> Steps<'a> {
        let next = (self.next_condition, Action::Intent(intent));
        self.list.push(next);
        Steps {
            list: self.list,
            timeout: self.timeout,
        }
    }

    pub fn then_view(mut self, data: ViewableData) -> Steps<'a> {
        let next = (self.next_condition, Action::View(data));
        self.list.push(next);
        Steps {
            list: self.list,
            timeout: self.timeout,
        }
    }

    pub fn then_view_with<F>(mut self, mut func: F) -> Steps<'a>
    where
        F: FnMut() -> ViewableData + Send + 'a,
    {
        let next = (self.next_condition, Action::View(func()));
        self.list.push(next);
        Steps {
            list: self.list,
            timeout: self.timeout,
        }
    }

    pub fn then_stop(mut self) -> Steps<'a> {
        let next = (self.next_condition, Action::Stop);
        self.list.push(next);
        Steps {
            list: self.list,
            timeout: self.timeout,
        }
    }
}

impl<'a> Steps<'a> {
    pub fn start() -> StepsWNextCondition<'a> {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: Vec::new(),
            timeout: Duration::from_secs(1),
        }
    }

    pub fn start_w_timeout(timeout: Duration) -> StepsWNextCondition<'a> {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: Vec::new(),
            timeout,
        }
    }

    pub fn immediatly(self) -> StepsWNextCondition<'a> {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: self.list,
            timeout: self.timeout,
        }
    }

    pub fn after_data(self, update: DataUpdateVariant) -> StepsWNextCondition<'a> {
        StepsWNextCondition {
            next_condition: Condition::DataUpdate(update),
            list: self.list,
            timeout: self.timeout,
        }
    }

    pub fn after_data_and<F>(self, data: DataUpdateVariant, condition: F) -> StepsWNextCondition<'a>
    where
        F: FnMut(&DataUpdate) -> bool + Send + 'a,
    {
        StepsWNextCondition {
            next_condition: Condition::DataUpdateAndFnMut {
                update: data,
                func: Box::new(condition),
            },
            list: self.list,
            timeout: self.timeout,
        }
    }
}
