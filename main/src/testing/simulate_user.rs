use presenter::UserIntent;
use traits::{DataUpdate, DataUpdateVariant, SearchResult};

pub enum AppUpdateVariant {
    Exit,
    Error,
    SearchResults,
}

pub enum Condition {
    None,
    Sleep(std::time::Duration),
    AppUpdate(AppUpdateVariant),
    DataUpdate(DataUpdateVariant),
}

pub enum ViewableData {
    PodcastList,
    Episode,
}

pub enum Action {
    View(ViewableData),
    Intent(UserIntent),
    Stop,
}

pub struct Steps {
    pub list: Vec<(Condition, Action)>,
}

pub struct StepsWContext<T> {
    list: Vec<(Condition, Action)>,
    context: T,
}

pub struct StepsWNextCondition {
    next_condition: Condition,
    list: Vec<(Condition, Action)>,
}

pub struct StepsWNextConditionAndContext<T> {
    next_condition: Condition,
    list: Vec<(Condition, Action)>,
    context: T,
}

impl StepsWNextCondition {
    pub fn then_do(mut self, intent: UserIntent) -> Steps {
        let next = (self.next_condition, Action::Intent(intent));
        self.list.push(next);
        Steps { list: self.list }
    }

    pub fn then_view(mut self, data: ViewableData) -> Steps {
        let next = (self.next_condition, Action::View(data));
        self.list.push(next);
        Steps { list: self.list }
    }

    pub fn then_stop(mut self) -> Steps {
        let next = (self.next_condition, Action::Stop);
        self.list.push(next);
        Steps { list: self.list }
    }
}

impl<T> StepsWNextConditionAndContext<T> {
    pub fn then(mut self, intent: UserIntent) -> StepsWContext<T> {
        let next = (self.next_condition, Action::Intent(intent));
        self.list.push(next);
        StepsWContext {
            list: self.list,
            context: self.context,
        }
    }

    pub fn then_view(mut self, data: ViewableData) -> Steps {
        let next = (self.next_condition, Action::View(data));
        self.list.push(next);
        Steps { list: self.list }
    }
}

impl Steps {
    pub fn start() -> StepsWNextCondition {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: Vec::new(),
        }
    }

    pub fn start_with_context<T>(context: T) -> StepsWNextConditionAndContext<T> {
        StepsWNextConditionAndContext {
            next_condition: Condition::None,
            list: Vec::new(),
            context,
        }
    }

    pub fn after_data(self, update: DataUpdateVariant) -> StepsWNextCondition {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: self.list,
        }
    }

    pub fn after_data_and<F>(self, data: DataUpdateVariant, condition: F) -> StepsWNextCondition
    where
        F: FnMut(&DataUpdate) -> bool,
    {
        StepsWNextCondition {
            next_condition: Condition::None,
            list: self.list,
        }
    }

    fn sleep(self, duration: std::time::Duration) -> StepsWNextCondition {
        StepsWNextCondition {
            next_condition: Condition::Sleep(duration),
            list: self.list,
        }
    }
}

// impl<T> StepsWContext<T> {
//     fn after_data(self, update: DataUpdateVariant) -> StepsWNextCondition {
//         StepsWNextCondition {
//             next_condition: Condition::None,
//             list: self.list,
//         }
//     }
//
//     fn sleep(self, duration: std::time::Duration) -> StepsWNextCondition {
//         StepsWNextCondition {
//             next_condition: Condition::Sleep(duration),
//             list: self.list,
//         }
//     }
// }

