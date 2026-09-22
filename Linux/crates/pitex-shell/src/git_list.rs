//! A lazy GListModel: a million paths must not mean a million GObjects.
use git_core::{GitChange, GitStatus};
use glib::subclass::prelude::*;
use gtk4::{gio, glib, prelude::*};
use std::sync::Arc;

pub(crate) enum Row {
    Empty,
    Section { staged: bool, count: usize },
    Change(GitChange),
}

mod imp {
    use super::*;
    use gio::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct GitList {
        pub status: RefCell<Option<Arc<GitStatus>>>,
        pub items: RefCell<std::collections::HashMap<u32, glib::WeakRef<glib::BoxedAnyObject>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GitList {
        const NAME: &'static str = "PitexGitList";
        type Type = super::GitList;
        type Interfaces = (gio::ListModel,);
    }
    impl ObjectImpl for GitList {}
    impl ListModelImpl for GitList {
        fn item_type(&self) -> glib::Type {
            glib::BoxedAnyObject::static_type()
        }
        fn n_items(&self) -> u32 {
            self.status.borrow().as_ref().map_or(0, |s| {
                (s.staged.len() + s.unstaged.len() + 1 + usize::from(!s.staged.is_empty())) as u32
            })
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            let mut items = self.items.borrow_mut();
            if let Some(item) = items.get(&position).and_then(glib::WeakRef::upgrade) {
                return Some(item.upcast());
            }
            let item = glib::BoxedAnyObject::new(self.obj().row(position)?);
            // Preserve GListModel identity while callers hold an item; never
            // retain every visited row when scrolling through a huge repo.
            if items.len() >= 1024 {
                items.retain(|_, item| item.upgrade().is_some());
            }
            items.insert(position, item.downgrade());
            Some(item.upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct GitList(ObjectSubclass<imp::GitList>) @implements gio::ListModel;
}

impl GitList {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn set_status(&self, status: Option<Arc<GitStatus>>) {
        if match (&*self.imp().status.borrow(), &status) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        } {
            return;
        }
        let old_count = self.n_items();
        let old = self.imp().status.replace(status);
        self.imp().items.borrow_mut().clear();
        self.items_changed(0, old_count, self.n_items());
        // Releasing the last snapshot can free millions of strings too.
        if old.is_some() {
            std::thread::spawn(move || drop(old));
        }
    }

    pub(crate) fn row(&self, position: u32) -> Option<Row> {
        let snapshot = self.imp().status.borrow();
        let s = snapshot.as_ref()?;
        let mut index = position as usize;
        if s.staged.is_empty() && s.unstaged.is_empty() {
            return (index == 0).then_some(Row::Empty);
        }
        if !s.staged.is_empty() {
            if index == 0 {
                return Some(Row::Section {
                    staged: true,
                    count: s.staged.len(),
                });
            }
            index -= 1;
            if let Some(change) = s.staged.get(index) {
                return Some(Row::Change(change.clone()));
            }
            index -= s.staged.len();
        }
        if index == 0 {
            return Some(Row::Section {
                staged: false,
                count: s.unstaged.len(),
            });
        }
        s.unstaged.get(index - 1).cloned().map(Row::Change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_keep_identity_and_map_both_sections_without_eager_objects() {
        let list = GitList::new();
        let status = Arc::new(git_core::parse_status(
            "## main\0MM both.tex\0R  새 이름.tex\0old.tex\0?? 123.tex\0",
            "/repo",
        ));
        list.set_status(Some(status.clone()));
        assert_eq!(list.n_items(), 6);
        assert!(list.imp().items.borrow().is_empty());
        let first = list.item(1).unwrap();
        assert_eq!(list.item(1).unwrap(), first);
        for (position, path, staged) in [
            (1, "both.tex", true),
            (2, "새 이름.tex", true),
            (4, "both.tex", false),
            (5, "123.tex", false),
        ] {
            let Some(Row::Change(change)) = list.row(position) else {
                panic!("missing change")
            };
            assert_eq!((change.path.as_str(), change.staged), (path, staged));
        }
        list.set_status(Some(status));
        assert_eq!(
            list.item(1).unwrap(),
            first,
            "unchanged refresh must preserve item identity"
        );
        assert!(list.item(6).is_none());
        list.set_status(Some(Arc::new(git_core::parse_status(
            "## empty\0",
            "/repo",
        ))));
        assert_eq!(list.n_items(), 1);
        assert!(matches!(list.row(0), Some(Row::Empty)));
        list.set_status(None);
        assert_eq!(list.n_items(), 0);
    }
}
