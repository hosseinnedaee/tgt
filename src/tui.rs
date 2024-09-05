use crate::{
    action::Action,
    app_context::AppContext,
    app_error::AppError,
    component_name::ComponentName,
    components::{
        component_traits::Component, core_window::CoreWindow, status_bar::StatusBar,
        title_bar::TitleBar, SMALL_AREA_HEIGHT, SMALL_AREA_WIDTH,
    },
    event::Event,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::{collections::HashMap, hash::Hasher, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use std::hash::{Hash, DefaultHasher};

/// `Tui` is a struct that represents the main user interface for the
/// application. It is responsible for managing the layout and rendering of all
/// the components. It also handles the distribution of events and actions to
/// the appropriate components.
pub struct Tui {
    /// The application configuration.
    app_context: Arc<AppContext>,
    /// An optional unbounded sender that can send actions to be processed.
    action_tx: Option<UnboundedSender<Action>>,
    /// A hashmap of components that make up the user interface.
    components: HashMap<ComponentName, Box<dyn Component>>,
    hash_frame: Option<u64>,
}
/// Implement the `Tui` struct.
impl Tui {
    /// Create a new instance of the `Tui` struct.
    ///
    /// # Arguments
    /// * `app_context` - An Arc wrapped AppContext struct.
    ///
    /// # Returns
    /// * `Self` - The new instance of the `Tui` struct.
    pub fn new(app_context: Arc<AppContext>) -> Self {
        let components_iter: Vec<(ComponentName, Box<dyn Component>)> = vec![
            (
                ComponentName::TitleBar,
                TitleBar::new(Arc::clone(&app_context))
                    .with_name("Tgt")
                    .new_boxed(),
            ),
            (
                ComponentName::CoreWindow,
                CoreWindow::new(Arc::clone(&app_context))
                    .with_name("Core Window")
                    .new_boxed(),
            ),
            (
                ComponentName::StatusBar,
                StatusBar::new(Arc::clone(&app_context))
                    .with_name("Status Bar")
                    .new_boxed(),
            ),
        ];
        let action_tx = None;
        let components: HashMap<ComponentName, Box<dyn Component>> =
            components_iter.into_iter().collect();

        Tui {
            action_tx,
            components,
            app_context,
            hash_frame: None,
        }
    }
    /// Register an action handler that can send actions for processing if
    /// necessary.
    ///
    /// # Arguments
    /// * `tx` - An unbounded sender that can send actions.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    pub fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), AppError<Action>> {
        self.action_tx = Some(tx.clone());
        self.components
            .iter_mut()
            .try_for_each(|(_, component)| component.register_action_handler(tx.clone()))?;
        Ok(())
    }
    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    pub fn handle_events(
        &mut self,
        event: Option<Event>,
    ) -> Result<Option<Action>, AppError<Action>> {
        self.components
            .get_mut(&ComponentName::CoreWindow)
            .unwrap()
            .handle_events(event.clone())
    }
    /// Update the state of the component based on a received action.
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    pub fn update(&mut self, action: Action) {
        // We can not send the action only to the `CoreWindow` component because
        // the `StatusBar` component needs to know the area to render the size.
        self.components
            .iter_mut()
            .for_each(|(_, component)| component.update(action.clone()));
    }
    /// Render the user interface to the screen.
    ///
    /// # Arguments
    /// * `frame` - A mutable reference to the frame to be rendered.
    /// * `area` - A rectangular area to render the user interface within.
    ///
    /// # Returns
    /// * `Result<()>` - An Ok result or an error.
    pub fn draw(&mut self, frame: &mut ratatui::Frame<'_>, area: Rect) -> Result<(), AppError<()>> {
        if let Some(current_hash) = self.hash_frame {
            let mut s = DefaultHasher::new();
            frame.hash(&mut s);
            let new_hash = s.finish();
            if current_hash.cmp(&new_hash) == std::cmp::Ordering::Equal {
                return Ok(());
            }
        }
        self.components
            .get_mut(&ComponentName::StatusBar)
            .unwrap()
            .update(Action::UpdateArea(area));

        let core_window: &mut dyn std::any::Any =
            self.components.get_mut(&ComponentName::CoreWindow).unwrap();
        if let Some(core_window) = core_window.downcast_mut::<CoreWindow>() {
            core_window.with_small_area(area.width < SMALL_AREA_WIDTH);
        }

        let main_layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(if self.app_context.app_config().show_title_bar {
                    if area.height > SMALL_AREA_HEIGHT + 5 {
                        3
                    } else {
                        0
                    }
                } else {
                    0
                }),
                Constraint::Min(SMALL_AREA_HEIGHT),
                Constraint::Length(if self.app_context.app_config().show_status_bar {
                    if area.height > SMALL_AREA_HEIGHT + 5 {
                        3
                    } else {
                        0
                    }
                } else {
                    0
                }),
            ],
        )
        .split(area);

        self.components
            .get_mut(&ComponentName::TitleBar)
            .unwrap_or_else(|| {
                tracing::error!("Failed to get component: {}", ComponentName::TitleBar);
                panic!("Failed to get component: {}", ComponentName::TitleBar)
            })
            .draw(frame, main_layout[0])?;

        self.components
            .get_mut(&ComponentName::CoreWindow)
            .unwrap_or_else(|| {
                tracing::error!("Failed to get component: {}", ComponentName::CoreWindow);
                panic!("Failed to get component: {}", ComponentName::CoreWindow)
            })
            .draw(frame, main_layout[1])?;

        self.components
            .get_mut(&ComponentName::StatusBar)
            .unwrap_or_else(|| {
                tracing::error!("Failed to get component: {}", ComponentName::StatusBar);
                panic!("Failed to get component: {}", ComponentName::StatusBar)
            })
            .draw(frame, main_layout[2])?;

        let mut s = DefaultHasher::new();
        frame.hash(&mut s);
        self.hash_frame = Some(s.finish());

        Ok(())
    }
}
