use gpui::{
    Animation, AnimationExt, AnyElement, AnyView, App, AppContext, Bounds, Context, Element,
    ElementId, Entity, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Render,
    Window, div, prelude::*, px,
};
use std::{panic::Location, time::Duration};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PageAnimation {
    #[default]
    None,
    Fade,
    Slide,
    SlideFade,
    SlideUp,
    SlideUpFade,
    Scale,
    ScaleFade,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PageOptions {
    pub(crate) title: Option<&'static str>,
    pub(crate) omit_root_layout: bool,
    open: Option<PageAnimation>,
    close: Option<PageAnimation>,
    duration: Option<Duration>,
    easing: Option<fn(f32) -> f32>,
}

impl PageOptions {
    pub fn without_root_layout(mut self) -> Self {
        self.omit_root_layout = true;
        self
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn on_open(mut self, animation: PageAnimation) -> Self {
        self.open = Some(animation);
        self
    }

    pub fn on_close(mut self, animation: PageAnimation) -> Self {
        self.close = Some(animation);
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn easing(mut self, easing: fn(f32) -> f32) -> Self {
        self.easing = Some(easing);
        self
    }

    pub(crate) fn effect(self, config: PagesConfig, opening: bool) -> Option<Effect> {
        let animation = if opening { self.open } else { self.close }.unwrap_or(config.animation);
        let duration = self.duration.unwrap_or(config.duration);
        (animation != PageAnimation::None && !duration.is_zero()).then_some(Effect {
            animation,
            duration,
            easing: self.easing.unwrap_or(config.easing),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PagesConfig {
    animation: PageAnimation,
    duration: Duration,
    easing: fn(f32) -> f32,
}

impl Default for PagesConfig {
    fn default() -> Self {
        Self {
            animation: PageAnimation::None,
            duration: Duration::from_millis(250),
            easing: |progress| 1.0 - (1.0 - progress).powi(3),
        }
    }
}

impl PagesConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_animation(mut self, animation: PageAnimation) -> Self {
        self.animation = animation;
        self
    }

    pub fn default_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn default_easing(mut self, easing: fn(f32) -> f32) -> Self {
        self.easing = easing;
        self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Effect {
    animation: PageAnimation,
    duration: Duration,
    easing: fn(f32) -> f32,
}

pub(crate) struct Transition {
    current: AnyView,
    opening: Option<Effect>,
    outgoing: Option<(AnyView, Effect)>,
}

impl Transition {
    pub(crate) fn mount(
        current: AnyView,
        opening: Option<Effect>,
        outgoing: Option<(AnyView, Effect)>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| {
            if let Some(effect) = opening {
                Self::schedule_end(effect.duration, true, cx);
            }
            if let Some((_, effect)) = &outgoing {
                Self::schedule_end(effect.duration, false, cx);
            }
            Self {
                current,
                opening,
                outgoing,
            }
        })
    }

    fn schedule_end(duration: Duration, opening: bool, cx: &Context<Self>) {
        let timer = cx.background_executor().timer(duration);
        cx.spawn(async move |transition, cx| {
            timer.await;
            let _ = transition.update(cx, |transition, cx| {
                if opening {
                    transition.opening = None;
                } else {
                    transition.outgoing = None;
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn finish(&mut self, cx: &mut Context<Self>) {
        self.opening = None;
        self.outgoing = None;
        cx.notify();
    }
}

impl Render for Transition {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .children(self.outgoing.as_ref().map(|(view, effect)| {
                div()
                    .absolute()
                    .inset_0()
                    .child(animated(view.clone(), *effect, false))
            }))
            .child(match self.opening {
                Some(effect) => animated(self.current.clone(), effect, true),
                None => div()
                    .size_full()
                    .occlude()
                    .child(self.current.clone())
                    .into_any_element(),
            })
    }
}

fn animated(view: AnyView, effect: Effect, opening: bool) -> AnyElement {
    let element = if opening {
        div().size_full().occlude()
    } else {
        div().size_full()
    };
    element
        .with_animation(
            if opening {
                "page-opening"
            } else {
                "page-closing"
            },
            Animation::new(effect.duration).with_easing(effect.easing),
            move |mut element, progress| {
                let visibility = if opening { progress } else { 1.0 - progress }.clamp(0.0, 1.0);
                if matches!(
                    effect.animation,
                    PageAnimation::Fade
                        | PageAnimation::SlideFade
                        | PageAnimation::SlideUpFade
                        | PageAnimation::ScaleFade
                ) {
                    element = element.opacity(visibility);
                }
                if matches!(
                    effect.animation,
                    PageAnimation::Slide | PageAnimation::SlideFade
                ) {
                    let direction = if opening { 1.0 } else { -1.0 };
                    element = element
                        .relative()
                        .left(px(24.0 * direction * (1.0 - visibility)));
                }
                if matches!(
                    effect.animation,
                    PageAnimation::SlideUp | PageAnimation::SlideUpFade
                ) {
                    let direction = if opening { 1.0 } else { -1.0 };
                    element = element
                        .relative()
                        .top(px(24.0 * direction * (1.0 - visibility)));
                }
                if matches!(
                    effect.animation,
                    PageAnimation::Scale | PageAnimation::ScaleFade
                ) {
                    element.child(RemScale {
                        child: view.clone().into_any_element(),
                        factor: 0.94 + 0.06 * visibility,
                    })
                } else {
                    element.child(view.clone())
                }
            },
        )
        .into_any_element()
}

struct RemScale {
    child: AnyElement,
    factor: f32,
}

impl IntoElement for RemScale {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RemScale {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let rem = window.rem_size() * self.factor;
        (
            window.with_rem_size(Some(rem), |window| self.child.request_layout(window, cx)),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let rem = window.rem_size() * self.factor;
        window.with_rem_size(Some(rem), |window| {
            self.child.prepaint(window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let rem = window.rem_size() * self.factor;
        window.with_rem_size(Some(rem), |window| self.child.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_options_override_global_defaults() {
        let config = PagesConfig::new().default_animation(PageAnimation::SlideFade);
        let page = PageOptions::default().on_close(PageAnimation::None);
        assert!(page.effect(config, false).is_none());
        let opening = page.effect(config, true).expect("global opening effect");
        assert_eq!(opening.animation, PageAnimation::SlideFade);
        assert_eq!(opening.duration, Duration::from_millis(250));
        let page = page
            .on_open(PageAnimation::ScaleFade)
            .duration(Duration::from_millis(90));
        let opening = page.effect(config, true).expect("page opening effect");
        assert_eq!(opening.animation, PageAnimation::ScaleFade);
        assert_eq!(opening.duration, Duration::from_millis(90));
    }
}
