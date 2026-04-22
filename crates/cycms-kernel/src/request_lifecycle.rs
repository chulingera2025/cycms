use axum::extract::Request;
use axum::response::Response;
use cycms_host_types::HostRequestTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    RequestReceived,
    RouteMatched,
    CompatSpaFallback,
    BeforeSend,
}

impl LifecyclePhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestReceived => "request_received",
            Self::RouteMatched => "route_matched",
            Self::CompatSpaFallback => "compat_spa_fallback",
            Self::BeforeSend => "before_send",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleTrace {
    pub target: HostRequestTarget,
    pub phases: Vec<LifecyclePhase>,
}

impl LifecycleTrace {
    #[must_use]
    pub fn new(path: String) -> Self {
        Self {
            target: HostRequestTarget { path },
            phases: vec![LifecyclePhase::RequestReceived],
        }
    }

    pub fn push(&mut self, phase: LifecyclePhase) {
        self.phases.push(phase);
    }

    #[must_use]
    pub fn header_value(&self) -> String {
        self.phases
            .iter()
            .map(|phase| phase.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug)]
pub struct PublicLifecycleOutcome {
    pub response: Option<Response>,
    pub trace: LifecycleTrace,
}

#[derive(Debug, Clone, Default)]
pub struct DefaultRequestLifecycleEngine;

impl DefaultRequestLifecycleEngine {
    #[must_use]
    pub fn execute_public_request(&self, request: &Request) -> PublicLifecycleOutcome {
        let mut trace = LifecycleTrace::new(request.uri().path().to_owned());
        trace.push(LifecyclePhase::RouteMatched);

        // Phase 3 foundation only captures the host-managed execution trace.
        // Route ownership and page rendering will attach a concrete response once
        // HostRegistry and the render pipeline are wired into the kernel.
        PublicLifecycleOutcome {
            response: None,
            trace,
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;

    use super::*;

    #[test]
    fn default_public_lifecycle_starts_trace_without_response() {
        let request = Request::builder()
            .uri("/posts/hello-world")
            .body(Body::empty())
            .unwrap();

        let outcome = DefaultRequestLifecycleEngine.execute_public_request(&request);

        assert!(outcome.response.is_none());
        assert_eq!(outcome.trace.target.path, "/posts/hello-world");
        assert_eq!(
            outcome.trace.phases,
            vec![
                LifecyclePhase::RequestReceived,
                LifecyclePhase::RouteMatched
            ]
        );
    }

    #[test]
    fn lifecycle_trace_header_value_is_deterministic() {
        let mut trace = LifecycleTrace::new("/".to_owned());
        trace.push(LifecyclePhase::RouteMatched);
        trace.push(LifecyclePhase::CompatSpaFallback);
        trace.push(LifecyclePhase::BeforeSend);

        assert_eq!(
            trace.header_value(),
            "request_received,route_matched,compat_spa_fallback,before_send"
        );
    }
}
