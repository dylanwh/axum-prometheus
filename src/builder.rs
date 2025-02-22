use std::marker::PhantomData;

use metrics_exporter_prometheus::PrometheusHandle;

use crate::{PrometheusMetricLayer, Traffic};

#[doc(hidden)]
mod sealed {
    use super::{LayerOnly, Paired};
    pub trait Sealed {}
    impl Sealed for LayerOnly {}
    impl Sealed for Paired {}
}
pub trait MetricBuilderState: sealed::Sealed {}

pub enum Paired {}
pub enum LayerOnly {}
impl MetricBuilderState for Paired {}
impl MetricBuilderState for LayerOnly {}

/// A builder for [`PrometheusMetricLayer`] that enables further customizations,
/// such as ignoring or masking routes and defining customized [`PrometheusHandle`]s.
///
/// ## Example
/// ```rust,no_run
/// use axum_prometheus::PrometheusMetricLayerBuilder;
///
/// let (metric_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
///     .with_ignore_patterns(&["/metrics", "/sensitive"])
///     .with_group_patterns_as("/foo", &["/foo/:bar", "/foo/:bar/:baz"])
///     .with_group_patterns_as("/bar", &["/auth/*path"])
///     .with_default_metrics()
///     .build_pair();
/// ```
#[derive(Clone, Default)]
pub struct PrometheusMetricLayerBuilder<'a, S: MetricBuilderState> {
    pub(crate) traffic: Traffic<'a>,
    pub(crate) metric_handle: Option<PrometheusHandle>,
    pub(crate) _marker: PhantomData<S>,
}

impl<'a, S> PrometheusMetricLayerBuilder<'a, S>
where
    S: MetricBuilderState,
{
    /// Skip reporting a specific route pattern to Prometheus.
    ///
    /// In the following example
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     .with_ignore_pattern("/metrics")
    ///     .build();
    /// ```
    /// any request that's URI path matches "/metrics" will be skipped altogether
    /// when reporting to Prometheus.
    ///
    /// Supports the same features as `axum`'s Router.
    ///
    ///  _Note that ignore patterns always checked before any other group pattern rule is applied
    /// and it short-circuits if a certain route is ignored._
    pub fn with_ignore_pattern(mut self, ignore_pattern: &'a str) -> Self {
        self.traffic.with_ignore_pattern(ignore_pattern);
        self
    }

    /// Skip reporting a collection of route patterns to Prometheus.
    ///
    /// Equivalent with calling [`with_ignore_pattern`] repeatedly.
    ///
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     .with_ignore_patterns(&["/foo", "/bar/:baz"])
    ///     .build();
    /// ```
    ///
    /// Supports the same features as `axum`'s Router.
    ///
    ///  _Note that ignore patterns always checked before any other group pattern rule is applied
    /// and it short-circuits if a certain route is ignored._
    ///
    /// [`with_ignore_pattern`]: crate::PrometheusMetricLayerBuilder::with_ignore_pattern
    pub fn with_ignore_patterns(mut self, ignore_patterns: &'a [&'a str]) -> Self {
        self.traffic.with_ignore_patterns(ignore_patterns);
        self
    }

    /// Group matching route patterns and report them under the given (arbitrary) endpoint.
    ///
    /// This feature is commonly useful for parametrized routes. Let's say you have these two routes:
    ///  - `/foo/:bar`
    ///  - `/foo/:bar/:baz`
    ///
    /// By default every unique request URL path gets reported with different endpoint label.
    /// This feature allows you to report these under a custom endpoint, for instance `/foo`:
    ///
    /// ```rust
    /// use axum_prometheus::PrometheusMetricLayerBuilder;
    ///
    /// let metric_layer = PrometheusMetricLayerBuilder::new()
    ///     // the choice of "/foo" is arbitrary
    ///     .with_group_patterns_as("/foo", &["/foo/:bar", "foo/:bar/:baz"])
    ///     .build();
    /// ```
    pub fn with_group_patterns_as(
        mut self,
        group_pattern: &'a str,
        patterns: &'a [&'a str],
    ) -> Self {
        self.traffic.with_group_patterns_as(group_pattern, patterns);
        self
    }
}

impl<'a> PrometheusMetricLayerBuilder<'a, LayerOnly> {
    /// Initialize the builder.
    pub fn new() -> PrometheusMetricLayerBuilder<'a, LayerOnly> {
        PrometheusMetricLayerBuilder {
            _marker: PhantomData,
            traffic: Traffic::new(),
            metric_handle: None,
        }
    }

    /// Attach the default [`PrometheusHandle`] to the builder. This is similar to
    /// initializing with [`PrometheusMetricLayer::pair`].
    ///
    /// After calling this function you can finalize with the [`build_pair`] method, and
    /// can no longer call [`build`].
    ///
    /// [`build`]: crate::PrometheusMetricLayerBuilder::build
    /// [`build_pair`]: crate::PrometheusMetricLayerBuilder::build_pair
    pub fn with_default_metrics(mut self) -> PrometheusMetricLayerBuilder<'a, Paired> {
        self.metric_handle = Some(PrometheusMetricLayer::make_default_handle());
        PrometheusMetricLayerBuilder::<'_, Paired>::from_layer_only(self)
    }

    /// Attach a custom [`PrometheusHandle`] to the builder that's returned from the passed
    /// in closure.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use axum_prometheus::{
    ///        PrometheusMetricLayerBuilder, AXUM_HTTP_REQUESTS_DURATION_SECONDS, SECONDS_DURATION_BUCKETS,
    /// };
    /// use metrics_exporter_prometheus::{Matcher, PrometheusBuilder};
    ///
    /// let (metric_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
    ///     .with_metrics_from_fn(|| {
    ///         PrometheusBuilder::new()
    ///             .set_buckets_for_metric(
    ///                 Matcher::Full(AXUM_HTTP_REQUESTS_DURATION_SECONDS.to_string()),
    ///                 SECONDS_DURATION_BUCKETS,
    ///             )
    ///             .unwrap()
    ///             .install_recorder()
    ///             .unwrap()
    ///     })
    ///     .build_pair();
    /// ```
    /// After calling this function you can finalize with the [`build_pair`] method, and
    /// can no longer call [`build`].
    ///
    /// [`build`]: crate::PrometheusMetricLayerBuilder::build
    /// [`build_pair`]: crate::PrometheusMetricLayerBuilder::build_pair
    pub fn with_metrics_from_fn(
        mut self,
        f: impl FnOnce() -> PrometheusHandle,
    ) -> PrometheusMetricLayerBuilder<'a, Paired> {
        self.metric_handle = Some(f());
        PrometheusMetricLayerBuilder::<'_, Paired>::from_layer_only(self)
    }

    /// Finalize the builder and get the [`PrometheusMetricLayer`] out of it.
    pub fn build(self) -> PrometheusMetricLayer<'a> {
        PrometheusMetricLayer::from_builder(self)
    }
}

impl<'a> PrometheusMetricLayerBuilder<'a, Paired> {
    pub(crate) fn from_layer_only(layer_only: PrometheusMetricLayerBuilder<'a, LayerOnly>) -> Self {
        PrometheusMetricLayerBuilder {
            _marker: PhantomData,
            traffic: layer_only.traffic,
            metric_handle: layer_only.metric_handle,
        }
    }
    /// Finalize the builder and get out the [`PrometheusMetricLayer`] and the
    /// [`PrometheusHandle`] out of it as a tuple.
    pub fn build_pair(self) -> (PrometheusMetricLayer<'a>, PrometheusHandle) {
        PrometheusMetricLayer::pair_from_builder(self)
    }
}
