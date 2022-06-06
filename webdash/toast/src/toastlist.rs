use super::*;

#[derive(Debug, PartialEq, Clone)]
pub struct ToastList {
    // Consider Rc for cheaper clone
    pub toasts: BTreeMap<usize, (ToastType, bool)>,
    counter: usize,
}

impl Reducible for ToastList {
    type Action = ToastAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut toasts = self.toasts.clone();
        let mut counter = self.counter;

        match action {
            ToastAction::Toast(t) => {
                counter += 1;
                toasts.insert(counter, (t, false));
            },
            ToastAction::Delete(tid) => {
                toasts.remove(&tid);
            },
            ToastAction::Fade(tid) => {
                if let Some(t) = toasts.get_mut(&tid) {
                    t.1 = true
                }
            }
        };

        Self {
            toasts,
            counter,
        }.into()
    }
}

impl ToastList {
    pub fn new() -> Self {
        Self {
            toasts: BTreeMap::new(),
            counter: 0,
        }
    }
}
