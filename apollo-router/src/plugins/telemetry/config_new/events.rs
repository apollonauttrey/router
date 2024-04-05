use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::Deserialize;
use tower::BoxError;

use super::instruments::Instrumented;
use super::Selector;
use super::Selectors;
use crate::plugins::telemetry::config_new::attributes::RouterAttributes;
use crate::plugins::telemetry::config_new::attributes::SubgraphAttributes;
use crate::plugins::telemetry::config_new::attributes::SupergraphAttributes;
use crate::plugins::telemetry::config_new::conditions::Condition;
use crate::plugins::telemetry::config_new::extendable::Extendable;
use crate::plugins::telemetry::config_new::selectors::RouterSelector;
use crate::plugins::telemetry::config_new::selectors::SubgraphSelector;
use crate::plugins::telemetry::config_new::selectors::SupergraphSelector;
use crate::services::router;
use crate::services::subgraph;
use crate::services::supergraph;
use crate::Context;

/// Events are
#[derive(Deserialize, JsonSchema, Clone, Default, Debug)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct Events {
    /// Router service events
    router: Extendable<RouterEventsConfig, Event<RouterAttributes, RouterSelector>>,
    /// Subgraph service events
    supergraph: Extendable<SupergraphEventsConfig, Event<SupergraphAttributes, SupergraphSelector>>,
    /// Supergraph service events
    subgraph: Extendable<SubgraphEventsConfig, Event<SubgraphAttributes, SubgraphSelector>>,
}

impl Events {
    pub(crate) fn new_router_events(&self) -> RouterEvents {
        let custom_events = self
            .router
            .custom
            .iter()
            .map(|(event_name, event_cfg)| CustomEvent {
                inner: Mutex::new(CustomEventInner {
                    name: event_name.clone(),
                    level: event_cfg.level,
                    event_on: event_cfg.on,
                    message: event_cfg.message.clone(),
                    selectors: event_cfg.attributes.clone().into(),
                    condition: event_cfg.condition.clone(),
                    attributes: Vec::new(),
                }),
            })
            .collect();

        RouterEvents {
            request: self.router.attributes.request,
            response: self.router.attributes.response,
            error: self.router.attributes.error,
            custom: custom_events,
        }
    }

    pub(crate) fn new_supergraph_events(&self) -> SupergraphEvents {
        let custom_events = self
            .supergraph
            .custom
            .iter()
            .map(|(event_name, event_cfg)| CustomEvent {
                inner: Mutex::new(CustomEventInner {
                    name: event_name.clone(),
                    level: event_cfg.level,
                    event_on: event_cfg.on,
                    message: event_cfg.message.clone(),
                    selectors: event_cfg.attributes.clone().into(),
                    condition: event_cfg.condition.clone(),
                    attributes: Vec::new(),
                }),
            })
            .collect();

        SupergraphEvents {
            request: self.supergraph.attributes.request,
            response: self.supergraph.attributes.response,
            error: self.supergraph.attributes.error,
            custom: custom_events,
        }
    }

    pub(crate) fn new_subgraph_events(&self) -> SubgraphEvents {
        let custom_events = self
            .subgraph
            .custom
            .iter()
            .map(|(event_name, event_cfg)| CustomEvent {
                inner: Mutex::new(CustomEventInner {
                    name: event_name.clone(),
                    level: event_cfg.level,
                    event_on: event_cfg.on,
                    message: event_cfg.message.clone(),
                    selectors: event_cfg.attributes.clone().into(),
                    condition: event_cfg.condition.clone(),
                    attributes: Vec::new(),
                }),
            })
            .collect();

        SubgraphEvents {
            request: self.subgraph.attributes.request,
            response: self.subgraph.attributes.response,
            error: self.subgraph.attributes.error,
            custom: custom_events,
        }
    }
}

pub(crate) type RouterEvents =
    CustomEvents<router::Request, router::Response, RouterAttributes, RouterSelector>;

pub(crate) type SupergraphEvents = CustomEvents<
    supergraph::Request,
    supergraph::Response,
    SupergraphAttributes,
    SupergraphSelector,
>;

pub(crate) type SubgraphEvents =
    CustomEvents<subgraph::Request, subgraph::Response, SubgraphAttributes, SubgraphSelector>;

// pub(crate) struct RouterEvents {
//     request: EventLevel,
//     response: EventLevel,
//     error: EventLevel,
//     custom: Vec<CustomEvent<router::Request, router::Response, RouterAttributes, RouterSelector>>,
// }

// impl Instrumented for RouterEvents {
//     type Request = router::Request;
//     type Response = router::Response;

//     fn on_request(&self, request: &Self::Request) {
//         if self.request != EventLevel::Off {
//             let mut attrs = HashMap::new();
//             attrs.insert(
//                 "headers".to_string(),
//                 format!("{:?}", request.router_request.headers()),
//             );
//             attrs.insert(
//                 "method".to_string(),
//                 format!("{}", request.router_request.method()),
//             );
//             attrs.insert(
//                 "uri".to_string(),
//                 format!("{}", request.router_request.uri()),
//             );
//             attrs.insert(
//                 "version".to_string(),
//                 format!("{:?}", request.router_request.version()),
//             );
//             attrs.insert(
//                 "body".to_string(),
//                 format!("{:?}", request.router_request.body()),
//             );
//             log_level(self.request, "router.request", &attrs, "");
//         }
//         for custom_event in &self.custom {
//             custom_event.on_request(request);
//         }
//     }

//     fn on_response(&self, response: &Self::Response) {
//         if self.response != EventLevel::Off {
//             let mut attrs = HashMap::new();
//             attrs.insert(
//                 "headers".to_string(),
//                 format!("{:?}", response.response.headers()),
//             );
//             attrs.insert(
//                 "status".to_string(),
//                 format!("{}", response.response.status()),
//             );
//             attrs.insert(
//                 "version".to_string(),
//                 format!("{:?}", response.response.version()),
//             );
//             attrs.insert(
//                 "body".to_string(),
//                 format!("{:?}", response.response.body()),
//             );
//             log_level(self.response, "router.response", &attrs, "");
//         }
//         for custom_event in &self.custom {
//             custom_event.on_response(response);
//         }
//     }

//     fn on_error(&self, error: &BoxError, ctx: &Context) {
//         if self.error != EventLevel::Off {
//             let mut attrs = HashMap::new();
//             attrs.insert("error".to_string(), error.to_string());
//             log_level(self.error, "router.error", &attrs, "");
//         }
//         for custom_event in &self.custom {
//             custom_event.on_error(error, ctx);
//         }
//     }
// }

pub(crate) struct CustomEvents<Request, Response, Attributes, Sel>
where
    Attributes: Selectors<Request = Request, Response = Response> + Default,
    Sel: Selector<Request = Request, Response = Response> + Debug,
{
    request: EventLevel,
    response: EventLevel,
    error: EventLevel,
    custom: Vec<CustomEvent<Request, Response, Attributes, Sel>>,
}

impl Instrumented
    for CustomEvents<router::Request, router::Response, RouterAttributes, RouterSelector>
{
    type Request = router::Request;
    type Response = router::Response;

    fn on_request(&self, request: &Self::Request) {
        if self.request != EventLevel::Off {
            let mut attrs = HashMap::with_capacity(5);
            attrs.insert(
                "http.request.headers".to_string(),
                format!("{:?}", request.router_request.headers()),
            );
            attrs.insert(
                "http.request.method".to_string(),
                format!("{}", request.router_request.method()),
            );
            attrs.insert(
                "http.request.uri".to_string(),
                format!("{}", request.router_request.uri()),
            );
            attrs.insert(
                "http.request.version".to_string(),
                format!("{:?}", request.router_request.version()),
            );
            attrs.insert(
                "http.request.body".to_string(),
                format!("{:?}", request.router_request.body()),
            );
            log_event(self.request, "router.request", &attrs, "");
        }
        for custom_event in &self.custom {
            custom_event.on_request(request);
        }
    }

    fn on_response(&self, response: &Self::Response) {
        if self.response != EventLevel::Off {
            let mut attrs = HashMap::with_capacity(4);
            attrs.insert(
                "http.response.headers".to_string(),
                format!("{:?}", response.response.headers()),
            );
            attrs.insert(
                "http.response.status".to_string(),
                format!("{}", response.response.status()),
            );
            attrs.insert(
                "http.response.version".to_string(),
                format!("{:?}", response.response.version()),
            );
            attrs.insert(
                "http.response.body".to_string(),
                format!("{:?}", response.response.body()),
            );
            log_event(self.response, "router.response", &attrs, "");
        }
        for custom_event in &self.custom {
            custom_event.on_response(response);
        }
    }

    fn on_error(&self, error: &BoxError, ctx: &Context) {
        if self.error != EventLevel::Off {
            let mut attrs = HashMap::with_capacity(1);
            attrs.insert("error".to_string(), error.to_string());
            log_event(self.error, "router.error", &attrs, "");
        }
        for custom_event in &self.custom {
            custom_event.on_error(error, ctx);
        }
    }
}

impl Instrumented
    for CustomEvents<
        supergraph::Request,
        supergraph::Response,
        SupergraphAttributes,
        SupergraphSelector,
    >
{
    type Request = supergraph::Request;
    type Response = supergraph::Response;

    fn on_request(&self, request: &Self::Request) {
        if self.request != EventLevel::Off {
            let mut attrs = HashMap::new();
            attrs.insert(
                "http.request.headers".to_string(),
                format!("{:?}", request.supergraph_request.headers()),
            );
            attrs.insert(
                "http.request.method".to_string(),
                format!("{}", request.supergraph_request.method()),
            );
            attrs.insert(
                "http.request.uri".to_string(),
                format!("{}", request.supergraph_request.uri()),
            );
            attrs.insert(
                "http.request.version".to_string(),
                format!("{:?}", request.supergraph_request.version()),
            );
            attrs.insert(
                "http.request.body".to_string(),
                serde_json::to_string(request.supergraph_request.body()).unwrap_or_default(),
            );
            log_event(self.request, "supergraph.request", &attrs, "");
        }
        if self.response != EventLevel::Off {
            request
                .context
                .extensions()
                .lock()
                .insert(SupergraphEventResponseLevel(self.response));
        }
        for custom_event in &self.custom {
            custom_event.on_request(request);
        }
    }

    fn on_response(&self, response: &Self::Response) {
        for custom_event in &self.custom {
            custom_event.on_response(response);
        }
    }

    fn on_error(&self, error: &BoxError, ctx: &Context) {
        if self.error != EventLevel::Off {
            let mut attrs = HashMap::new();
            attrs.insert("error".to_string(), error.to_string());
            log_event(self.error, "supergraph.error", &attrs, "");
        }
        for custom_event in &self.custom {
            custom_event.on_error(error, ctx);
        }
    }
}

impl Instrumented
    for CustomEvents<subgraph::Request, subgraph::Response, SubgraphAttributes, SubgraphSelector>
{
    type Request = subgraph::Request;
    type Response = subgraph::Response;

    fn on_request(&self, request: &Self::Request) {
        if self.request != EventLevel::Off {
            request
                .context
                .extensions()
                .lock()
                .insert(SubgraphEventRequestLevel(self.request));
        }
        if self.response != EventLevel::Off {
            request
                .context
                .extensions()
                .lock()
                .insert(SubgraphEventResponseLevel(self.response));
        }
        for custom_event in &self.custom {
            custom_event.on_request(request);
        }
    }

    fn on_response(&self, response: &Self::Response) {
        for custom_event in &self.custom {
            custom_event.on_response(response);
        }
    }

    fn on_error(&self, error: &BoxError, ctx: &Context) {
        if self.error != EventLevel::Off {
            let mut attrs = HashMap::new();
            attrs.insert("error".to_string(), error.to_string());
            log_event(self.error, "subgraph.error", &attrs, "");
        }
        for custom_event in &self.custom {
            custom_event.on_error(error, ctx);
        }
    }
}

#[derive(Clone, Deserialize, JsonSchema, Debug, Default)]
#[serde(deny_unknown_fields, default)]
struct RouterEventsConfig {
    /// Log the router request
    request: EventLevel,
    /// Log the router response
    response: EventLevel,
    /// Log the router error
    error: EventLevel,
}

#[derive(Clone)]
pub(crate) struct SupergraphEventResponseLevel(pub(crate) EventLevel);
#[derive(Clone)]
pub(crate) struct SubgraphEventResponseLevel(pub(crate) EventLevel);
#[derive(Clone)]
pub(crate) struct SubgraphEventRequestLevel(pub(crate) EventLevel);

#[derive(Clone, Deserialize, JsonSchema, Debug, Default)]
#[serde(deny_unknown_fields, default)]
struct SupergraphEventsConfig {
    /// Log the supergraph request
    request: EventLevel,
    /// Log the supergraph response
    response: EventLevel,
    /// Log the supergraph error
    error: EventLevel,
}

#[derive(Clone, Deserialize, JsonSchema, Debug, Default)]
#[serde(deny_unknown_fields, default)]
struct SubgraphEventsConfig {
    /// Log the subgraph request
    request: EventLevel,
    /// Log the subgraph response
    response: EventLevel,
    /// Log the subgraph error
    error: EventLevel,
}

#[derive(Deserialize, JsonSchema, Clone, Debug, Default, PartialEq, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventLevel {
    Info,
    Warn,
    Error,
    #[default]
    Off,
}

/// An event that can be logged as part of a trace.
/// The event has an implicit `type` attribute that matches the name of the event in the yaml
/// and a message that can be used to provide additional information.
#[derive(Deserialize, JsonSchema, Clone, Debug)]
pub(crate) struct Event<A, E>
where
    A: Default + Debug,
    E: Debug,
{
    /// The log level of the event.
    level: EventLevel,

    /// The event message.
    message: Arc<String>,

    /// When to trigger the event.
    on: EventOn,

    /// The event attributes.
    #[serde(default = "Extendable::empty_arc::<A, E>")]
    attributes: Arc<Extendable<A, E>>,

    /// The event conditions.
    #[serde(default = "Condition::empty::<E>")]
    condition: Condition<E>,
}

/// When to trigger the event.
#[derive(Deserialize, JsonSchema, Clone, Debug, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventOn {
    /// Log the event on request
    Request,
    /// Log the event on response
    Response,
    /// Log the event on error
    Error,
}

pub(crate) struct CustomEvent<Request, Response, A, T>
where
    A: Selectors<Request = Request, Response = Response> + Default,
    T: Selector<Request = Request, Response = Response> + Debug,
{
    inner: Mutex<CustomEventInner<Request, Response, A, T>>,
}

struct CustomEventInner<Request, Response, A, T>
where
    A: Selectors<Request = Request, Response = Response> + Default,
    T: Selector<Request = Request, Response = Response> + Debug,
{
    name: String,
    level: EventLevel,
    event_on: EventOn,
    message: Arc<String>,
    selectors: Option<Arc<Extendable<A, T>>>,
    condition: Condition<T>,
    attributes: Vec<opentelemetry_api::KeyValue>,
}

impl<A, T, Request, Response> Instrumented for CustomEvent<Request, Response, A, T>
where
    A: Selectors<Request = Request, Response = Response> + Default,
    T: Selector<Request = Request, Response = Response> + Debug + Debug,
{
    type Request = Request;
    type Response = Response;

    fn on_request(&self, request: &Self::Request) {
        let mut inner = self.inner.lock();
        if inner.condition.evaluate_request(request) != Some(true)
            && inner.event_on == EventOn::Request
        {
            return;
        }
        if let Some(selectors) = &inner.selectors {
            inner.attributes = selectors.on_request(request);
        }

        if inner.event_on == EventOn::Request
            && inner.condition.evaluate_request(request) != Some(false)
        {
            inner.send_event();
        }
    }

    fn on_response(&self, response: &Self::Response) {
        let mut inner = self.inner.lock();
        if inner.event_on != EventOn::Response {
            return;
        }

        if !inner.condition.evaluate_response(response) {
            return;
        }
        if let Some(selectors) = &inner.selectors {
            let mut new_attributes = selectors.on_response(response);
            inner.attributes.append(&mut new_attributes);
        }

        inner.send_event();
    }

    fn on_error(&self, error: &BoxError, _ctx: &Context) {
        let mut inner = self.inner.lock();
        if inner.event_on != EventOn::Error {
            return;
        }
        if let Some(selectors) = &inner.selectors {
            let mut new_attributes = selectors.on_error(error);
            inner.attributes.append(&mut new_attributes);
        }

        inner.send_event();
    }
}

impl<A, T, Request, Response> CustomEventInner<Request, Response, A, T>
where
    A: Selectors<Request = Request, Response = Response> + Default,
    T: Selector<Request = Request, Response = Response> + Debug + Debug,
{
    #[inline]
    fn send_event(&self) {
        let attributes: HashMap<String, String> = self
            .attributes
            .iter()
            .map(|kv| (kv.key.to_string(), kv.value.to_string()))
            .collect();
        log_event(self.level, &self.name, &attributes, &self.message);
    }
}

#[inline]
pub(crate) fn log_event(
    level: EventLevel,
    kind: &str,
    attributes: &HashMap<String, String>,
    message: &str,
) {
    match level {
        EventLevel::Info => {
            ::tracing::info!(%kind, attributes = ?attributes, "{}", message);
        }
        EventLevel::Warn => ::tracing::warn!(%kind, attributes = ?attributes, "{}", message),
        EventLevel::Error => ::tracing::error!(%kind, attributes = ?attributes, "{}", message),
        EventLevel::Off => {}
    }
}
