use gloo::history::{BrowserHistory, History, HistoryListener, Location};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use dioxus_core::ScopeId;

pub struct RouterService {
    pub(crate) regen_route: Rc<dyn Fn(ScopeId)>,
    history: Rc<RefCell<BrowserHistory>>,
    registered_routes: RefCell<RouteSlot>,
    slots: Rc<RefCell<Vec<(ScopeId, String)>>>,
    root_found: Rc<Cell<bool>>,
    cur_root: RefCell<String>,
    listener: HistoryListener,
}

enum RouteSlot {
    Routes {
        // the partial route
        partial: String,

        // the total route
        total: String,

        // Connections to other routs
        rest: Vec<RouteSlot>,
    },
}

impl RouterService {
    pub fn new(regen_route: Rc<dyn Fn(ScopeId)>, root_scope: ScopeId) -> Self {
        let history = BrowserHistory::default();
        let location = history.location();
        let path = location.path();

        let slots: Rc<RefCell<Vec<(ScopeId, String)>>> = Default::default();

        let _slots = slots.clone();

        let root_found = Rc::new(Cell::new(false));
        let regen = regen_route.clone();
        let _root_found = root_found.clone();
        let listener = history.listen(move || {
            _root_found.set(false);
            // checking if the route is valid is cheap, so we do it
            for (slot, root) in _slots.borrow_mut().iter().rev() {
                log::trace!("regenerating slot {:?} for root '{}'", slot, root);
                regen(*slot);
            }
        });

        Self {
            registered_routes: RefCell::new(RouteSlot::Routes {
                partial: String::from("/"),
                total: String::from("/"),
                rest: Vec::new(),
            }),
            root_found,
            history: Rc::new(RefCell::new(history)),
            regen_route,
            slots,
            cur_root: RefCell::new(path.to_string()),
            listener,
        }
    }

    pub fn push_route(&self, route: &str) {
        log::trace!("Pushing route: {}", route);
        self.history.borrow_mut().push(route);
    }

    pub fn register_total_route(&self, route: String, scope: ScopeId, fallback: bool) {
        let clean = clean_route(route);
        log::trace!("Registered route '{}' with scope id {:?}", clean, scope);
        self.slots.borrow_mut().push((scope, clean));
    }

    pub fn should_render(&self, scope: ScopeId) -> bool {
        log::trace!("Should render scope id {:?}?", scope);
        if self.root_found.get() {
            log::trace!("  no - because root_found is true");
            return false;
        }

        let location = self.history.borrow().location();
        let path = location.path();
        log::trace!("  current path is '{}'", path);

        let roots = self.slots.borrow();

        let root = roots.iter().find(|(id, route)| id == &scope);

        // fallback logic
        match root {
            Some((_id, route)) => {
                log::trace!(
                    "  matched given scope id {:?} with route root '{}'",
                    scope,
                    route,
                );
                if route_matches_path(route, path) {
                    log::trace!("    and it matches the current path '{}'", path);
                    self.root_found.set(true);
                    true
                } else {
                    if route == "" {
                        log::trace!("    and the route is the root, so we will use that without a better match");
                        self.root_found.set(true);
                        true
                    } else {
                        log::trace!("    and the route '{}' is not the root nor does it match the current path", route);
                        false
                    }
                }
            }
            None => false,
        }
    }

    pub fn current_location(&self) -> Location {
        self.history.borrow().location().clone()
    }
}

fn clean_route(route: String) -> String {
    if route.as_str() == "/" {
        return route;
    }
    route.trim_end_matches('/').to_string()
}

fn clean_path(path: &str) -> &str {
    if path == "/" {
        return path;
    }
    path.trim_end_matches('/')
}

fn route_matches_path(route: &str, path: &str) -> bool {
    let route_pieces = route.split('/').collect::<Vec<_>>();
    let path_pieces = clean_path(path).split('/').collect::<Vec<_>>();

    log::trace!(
        "  checking route pieces {:?} vs path pieces {:?}",
        route_pieces,
        path_pieces,
    );

    if route_pieces.len() != path_pieces.len() {
        log::trace!("    the routes are different lengths");
        return false;
    }

    for (i, r) in route_pieces.iter().enumerate() {
        log::trace!("    checking route piece '{}' vs path", r);
        // If this is a parameter then it matches as long as there's
        // _any_thing in that spot in the path.
        if r.starts_with(':') {
            log::trace!(
                "      route piece '{}' starts with a colon so it matches anything",
                r,
            );
            continue;
        }
        log::trace!(
            "      route piece '{}' must be an exact match for path piece '{}'",
            r,
            path_pieces[i],
        );
        if path_pieces[i] != *r {
            return false;
        }
    }

    return true;
}

pub struct RouterCfg {
    initial_route: String,
}

impl RouterCfg {
    pub fn new(initial_route: String) -> Self {
        Self { initial_route }
    }
}
