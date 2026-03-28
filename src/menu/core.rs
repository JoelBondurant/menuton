use iced::Event;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::text::{self};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::keyboard;
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Shadow, Size, Vector};

use crate::colors::{BG_PRIMARY, BORDER_PRIMARY, SHADOW_PRIMARY, TEXT_PRIMARY, TEXT_SECONDARY};
use crate::fonts::MENU_FONT;
use crate::menu::geometry::{
	ARROW_GUTTER, BAR_ITEM_PADDING_X, Hit, ItemKind, LABEL_SIZE, MenuGeometry, PANEL_TEXT_OFFSET,
};
use crate::menu::interaction::{
	WidgetState, adjacent_root, focused_panel_items, is_menu_activation, navigation_direction,
	panel_items, selectable_item,
};
use crate::menu::{MenuItem, MenuRoot, MenuState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuMessage {
	ToggleRoot(&'static str),
	OpenRoot(&'static str),
	OpenSubmenu { depth: usize, id: &'static str },
	TrimPath(usize),
	Invoke(&'static str),
	Close,
}

impl MenuState {
	pub fn update(&mut self, message: MenuMessage) -> Option<&'static str> {
		match message {
			MenuMessage::ToggleRoot(id) => {
				if self.is_root_open(id) {
					self.close();
				} else {
					self.set_open_root(id);
				}

				None
			}
			MenuMessage::OpenRoot(id) => {
				if !self.is_root_open(id) {
					self.set_open_root(id);
				}

				None
			}
			MenuMessage::OpenSubmenu { depth, id } => {
				self.set_open_submenu(depth, id);
				None
			}
			MenuMessage::TrimPath(depth) => {
				self.trim_path(depth);
				None
			}
			MenuMessage::Invoke(id) => {
				self.close();
				Some(id)
			}
			MenuMessage::Close => {
				self.close();
				None
			}
		}
	}
}

pub struct MenuBar<'a> {
	roots: &'a [MenuRoot],
	state: &'a MenuState,
}

impl<'a> MenuBar<'a> {
	pub fn new(roots: &'a [MenuRoot], state: &'a MenuState) -> Self {
		Self { roots, state }
	}
}

impl<'a, Theme, Renderer> Widget<MenuMessage, Theme, Renderer> for MenuBar<'a>
where
	Renderer: text::Renderer<Font = iced::Font>,
{
	fn size(&self) -> Size<Length> {
		Size::new(Length::Fill, Length::Shrink)
	}

	fn layout(
		&mut self,
		_tree: &mut Tree,
		renderer: &Renderer,
		limits: &layout::Limits,
	) -> layout::Node {
		let geometry = MenuGeometry::new(self.roots, self.state, renderer, limits.max().width);

		layout::Node::new(limits.resolve(Length::Fill, Length::Shrink, geometry.size()))
	}

	fn draw(
		&self,
		tree: &Tree,
		renderer: &mut Renderer,
		_theme: &Theme,
		_style: &renderer::Style,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		viewport: &Rectangle,
	) {
		let widget_state = tree.state.downcast_ref::<WidgetState>();
		let geometry = MenuGeometry::new(self.roots, self.state, renderer, layout.bounds().width)
			.with_origin(Point::new(layout.bounds().x, layout.bounds().y));

		renderer.fill_quad(
			renderer::Quad {
				bounds: geometry.bar_bounds,
				..renderer::Quad::default()
			},
			Background::Color(bar_background()),
		);

		for root in &geometry.roots {
			let background = if self.state.is_root_open(root.id) {
				Some(bar_active())
			} else if cursor.is_over(root.bounds) {
				Some(bar_hover())
			} else {
				None
			};

			if let Some(color) = background {
				renderer.fill_quad(
					renderer::Quad {
						bounds: root.bounds,
						..renderer::Quad::default()
					},
					Background::Color(color),
				);
			}

			draw_label(
				renderer,
				root.label,
				root.bounds,
				text_color(),
				viewport,
				LabelAlignment::Bar,
			);
		}

		for panel in &geometry.panels {
			renderer.fill_quad(
				renderer::Quad {
					bounds: panel.bounds,
					border: Border {
						color: panel_border(),
						width: 1.0,
						radius: 4.0.into(),
					},
					shadow: Shadow {
						color: SHADOW_PRIMARY,
						offset: Vector::new(0.0, 3.0),
						blur_radius: 10.0,
					},
					snap: false,
				},
				Background::Color(panel_background()),
			);

			for item in &panel.items {
				match item.kind {
					ItemKind::Separator => {
						renderer.fill_quad(
							renderer::Quad {
								bounds: Rectangle {
									x: item.bounds.x + PANEL_TEXT_OFFSET,
									y: item.bounds.center_y(),
									width: item.bounds.width - PANEL_TEXT_OFFSET * 2.0,
									height: 1.0,
								},
								..renderer::Quad::default()
							},
							Background::Color(panel_border()),
						);
					}
					ItemKind::Action { label, .. } | ItemKind::Submenu { label, .. } => {
						let hovered = cursor.is_over(item.bounds)
							|| match item.kind {
								ItemKind::Action { id, .. } | ItemKind::Submenu { id, .. } => {
									widget_state.focused(id, item.depth)
								}
								ItemKind::Separator => false,
							};

						if hovered {
							renderer.fill_quad(
								renderer::Quad {
									bounds: item.bounds,
									..renderer::Quad::default()
								},
								Background::Color(panel_hover()),
							);
						}

						draw_label(
							renderer,
							label,
							item.bounds,
							text_color(),
							viewport,
							LabelAlignment::Panel,
						);

						if matches!(item.kind, ItemKind::Submenu { .. }) {
							draw_label(
								renderer,
								"▷",
								Rectangle {
									x: item.bounds.x + item.bounds.width - ARROW_GUTTER,
									..item.bounds
								},
								if hovered { text_color() } else { text_muted() },
								viewport,
								LabelAlignment::Arrow,
							);
						}
					}
				}
			}
		}
	}

	fn update(
		&mut self,
		tree: &mut Tree,
		event: &Event,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		renderer: &Renderer,
		_clipboard: &mut dyn Clipboard,
		shell: &mut Shell<'_, MenuMessage>,
		_viewport: &Rectangle,
	) {
		let widget_state = tree.state.downcast_mut::<WidgetState>();
		let geometry = MenuGeometry::new(self.roots, self.state, renderer, layout.bounds().width)
			.with_origin(Point::new(layout.bounds().x, layout.bounds().y));

		match event {
			Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if widget_state.keyboard_navigation {
					widget_state.keyboard_navigation = false;
					shell.request_redraw();
				}

				if let Some(hit) = geometry.hit_test(cursor) {
					match hit {
						Hit::Root(root) if self.state.open_root().is_some() => {
							shell.publish(MenuMessage::OpenRoot(root.id));
							shell.capture_event();
						}
						Hit::Root(_) => {
							shell.capture_event();
						}
						Hit::PanelItem(item) => match item.kind {
							ItemKind::Submenu { id, .. } => {
								shell.publish(MenuMessage::OpenSubmenu {
									depth: item.depth,
									id,
								});
								shell.capture_event();
							}
							ItemKind::Action { .. } => {
								shell.publish(MenuMessage::TrimPath(item.depth));
								shell.capture_event();
							}
							ItemKind::Separator => {
								shell.capture_event();
							}
						},
						Hit::Panel => {
							shell.capture_event();
						}
					}
				} else if self.state.open_root().is_some() && cursor.is_over(layout.bounds()) {
					shell.publish(MenuMessage::TrimPath(0));
					shell.capture_event();
				}
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if widget_state.keyboard_navigation {
					widget_state.keyboard_navigation = false;
					shell.request_redraw();
				}

				if let Some(hit) = geometry.hit_test(cursor) {
					match hit {
						Hit::Root(root) => {
							widget_state.focus_root_panel(self.roots, root.id);
							shell.publish(MenuMessage::ToggleRoot(root.id));
							shell.capture_event();
						}
						Hit::PanelItem(item) => match item.kind {
							ItemKind::Action { id, .. } => {
								if item.depth < widget_state.focus_path.len() {
									widget_state.focus_path[item.depth] = id;
									widget_state.focus_path.truncate(item.depth + 1);
								}
								shell.publish(MenuMessage::Invoke(id));
								shell.capture_event();
							}
							ItemKind::Submenu { id, .. } => {
								widget_state.sync(self.roots, self.state);
								if let Some(items) = panel_items(self.roots, self.state, item.depth)
								{
									widget_state.focus_submenu(items, item.depth, id);
								}
								shell.publish(MenuMessage::OpenSubmenu {
									depth: item.depth,
									id,
								});
								shell.capture_event();
							}
							ItemKind::Separator => {
								shell.capture_event();
							}
						},
						Hit::Panel => {
							shell.capture_event();
						}
					}
				} else if self.state.open_root().is_some() {
					widget_state.clear();
					shell.publish(MenuMessage::Close);
					shell.capture_event();
				}
			}
			Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
				let shift = modifiers.shift();

				match key.as_ref() {
					key if is_menu_activation(&key, *modifiers) => {
						if self.state.open_root().is_some() {
							widget_state.clear();
							shell.publish(MenuMessage::Close);
						} else if let Some(first_root) = self.roots.first() {
							widget_state.focus_root_panel(self.roots, first_root.id);
							shell.publish(MenuMessage::OpenRoot(first_root.id));
						}

						shell.request_redraw();
						shell.capture_event();
					}
					keyboard::key::Key::Named(keyboard::key::Named::Escape)
						if self.state.open_root().is_some() =>
					{
						widget_state.clear();
						shell.publish(MenuMessage::Close);
						shell.capture_event();
					}
					key if navigation_direction(&key, shift).is_some() => {
						if let Some(open_root) = self.state.open_root() {
							widget_state.keyboard_navigation = true;
							widget_state.sync(self.roots, self.state);
							let direction = navigation_direction(&key, shift)
								.expect("navigation key should map to a direction");

							if widget_state.focus_path.is_empty() {
								widget_state.focus_root_panel(self.roots, open_root);
							} else {
								widget_state.focus_current_panel(self.roots, self.state, direction);
							}

							shell.request_redraw();
							shell.capture_event();
						}
					}
					keyboard::key::Key::Named(keyboard::key::Named::ArrowRight) => {
						if let Some(open_root) = self.state.open_root() {
							widget_state.keyboard_navigation = true;
							widget_state.sync(self.roots, self.state);

							if let Some((depth, items)) =
								focused_panel_items(self.roots, self.state, widget_state)
								&& let Some(focused_id) =
									widget_state.focus_path.get(depth).copied()
								&& let Some(MenuItem::Submenu { id, .. }) =
									selectable_item(items, focused_id)
							{
								widget_state.focus_submenu(items, depth, id);
								shell.publish(MenuMessage::OpenSubmenu { depth, id });
							} else if let Some(next_root) = adjacent_root(self.roots, open_root, 1)
							{
								widget_state.focus_root_panel(self.roots, next_root.id);
								shell.publish(MenuMessage::OpenRoot(next_root.id));
							}

							shell.request_redraw();
							shell.capture_event();
						}
					}
					keyboard::key::Key::Named(keyboard::key::Named::ArrowLeft) => {
						if let Some(open_root) = self.state.open_root() {
							widget_state.keyboard_navigation = true;
							widget_state.sync(self.roots, self.state);

							if widget_state.focus_path.len() > 1 {
								widget_state.focus_path.pop();
								shell.publish(MenuMessage::TrimPath(
									widget_state.focus_path.len().saturating_sub(1),
								));
							} else if let Some(previous_root) =
								adjacent_root(self.roots, open_root, -1)
							{
								widget_state.focus_root_panel(self.roots, previous_root.id);
								shell.publish(MenuMessage::OpenRoot(previous_root.id));
							}

							shell.request_redraw();
							shell.capture_event();
						}
					}
					keyboard::key::Key::Named(keyboard::key::Named::Enter) => {
						if self.state.open_root().is_some() {
							widget_state.keyboard_navigation = true;
							widget_state.sync(self.roots, self.state);

							if let Some((depth, items)) =
								focused_panel_items(self.roots, self.state, widget_state)
								&& let Some(focused_id) =
									widget_state.focus_path.get(depth).copied()
							{
								match selectable_item(items, focused_id) {
									Some(MenuItem::Action { id, .. }) => {
										shell.publish(MenuMessage::Invoke(id));
										widget_state.clear();
									}
									Some(MenuItem::Submenu { id, .. }) => {
										widget_state.focus_submenu(items, depth, id);
										shell.publish(MenuMessage::OpenSubmenu { depth, id });
									}
									_ => {}
								}
							}

							shell.request_redraw();
							shell.capture_event();
						}
					}
					_ => {}
				}
			}
			_ => {}
		}
	}

	fn mouse_interaction(
		&self,
		_tree: &Tree,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		_viewport: &Rectangle,
		renderer: &Renderer,
	) -> mouse::Interaction {
		let geometry = MenuGeometry::new(self.roots, self.state, renderer, layout.bounds().width)
			.with_origin(Point::new(layout.bounds().x, layout.bounds().y));

		if geometry.hit_test(cursor).is_some() {
			mouse::Interaction::Pointer
		} else {
			mouse::Interaction::None
		}
	}

	fn tag(&self) -> tree::Tag {
		tree::Tag::of::<WidgetState>()
	}

	fn state(&self) -> tree::State {
		tree::State::new(WidgetState::default())
	}
}

impl<'a, Theme, Renderer> From<MenuBar<'a>> for Element<'a, MenuMessage, Theme, Renderer>
where
	Theme: 'a,
	Renderer: text::Renderer<Font = iced::Font> + 'a,
{
	fn from(menu: MenuBar<'a>) -> Self {
		Element::new(menu)
	}
}

fn draw_label<Renderer: text::Renderer<Font = iced::Font>>(
	renderer: &mut Renderer,
	label: &str,
	bounds: Rectangle,
	color: Color,
	viewport: &Rectangle,
	alignment: LabelAlignment,
) {
	let horizontal_alignment = match alignment {
		LabelAlignment::Arrow => text::Alignment::Center,
		_ => text::Alignment::Left,
	};

	let x = match alignment {
		LabelAlignment::Bar => bounds.x + BAR_ITEM_PADDING_X,
		LabelAlignment::Panel => bounds.x + PANEL_TEXT_OFFSET,
		LabelAlignment::Arrow => bounds.x,
	};

	renderer.fill_text(
		text::Text {
			content: label.to_owned(),
			bounds: Size::new(bounds.width, bounds.height),
			size: LABEL_SIZE,
			line_height: text::LineHeight::default(),
			font: MENU_FONT,
			align_x: horizontal_alignment,
			align_y: iced::alignment::Vertical::Center,
			shaping: text::Shaping::Basic,
			wrapping: text::Wrapping::None,
		},
		Point::new(x, bounds.center_y()),
		color,
		*viewport,
	);
}

fn bar_background() -> Color {
	BG_PRIMARY
}

fn bar_hover() -> Color {
	Color::from_rgba(TEXT_SECONDARY.r, TEXT_SECONDARY.g, TEXT_SECONDARY.b, 0.16)
}

fn bar_active() -> Color {
	Color::from_rgba(TEXT_SECONDARY.r, TEXT_SECONDARY.g, TEXT_SECONDARY.b, 0.28)
}

fn panel_background() -> Color {
	BG_PRIMARY
}

fn panel_border() -> Color {
	BORDER_PRIMARY
}

fn panel_hover() -> Color {
	Color::from_rgba(TEXT_SECONDARY.r, TEXT_SECONDARY.g, TEXT_SECONDARY.b, 0.22)
}

fn text_color() -> Color {
	TEXT_PRIMARY
}

fn text_muted() -> Color {
	TEXT_SECONDARY
}

#[derive(Debug, Clone, Copy)]
enum LabelAlignment {
	Bar,
	Panel,
	Arrow,
}

#[cfg(test)]
mod tests {
	use super::{MenuMessage, MenuState};

	#[test]
	fn toggling_a_root_reopens_cleanly() {
		let mut state = MenuState::default();

		assert_eq!(state.update(MenuMessage::ToggleRoot("file")), None);
		assert!(state.is_root_open("file"));

		assert_eq!(
			state.update(MenuMessage::OpenSubmenu {
				depth: 0,
				id: "export",
			}),
			None
		);
		assert!(state.is_submenu_open(0, "export"));

		assert_eq!(state.update(MenuMessage::ToggleRoot("edit")), None);
		assert!(state.is_root_open("edit"));
		assert!(!state.is_submenu_open(0, "export"));
	}

	#[test]
	fn invoking_an_action_closes_the_menu() {
		let mut state = MenuState::default();

		state.update(MenuMessage::ToggleRoot("file"));

		assert_eq!(
			state.update(MenuMessage::Invoke("file.open")),
			Some("file.open")
		);
		assert_eq!(state.open_root(), None);
	}

	#[test]
	fn trimming_a_path_keeps_parent_panels_open() {
		let mut state = MenuState::default();

		state.update(MenuMessage::ToggleRoot("file"));
		state.update(MenuMessage::OpenSubmenu {
			depth: 0,
			id: "export",
		});
		state.update(MenuMessage::OpenSubmenu {
			depth: 1,
			id: "png",
		});

		state.update(MenuMessage::TrimPath(1));

		assert!(state.is_submenu_open(0, "export"));
		assert!(!state.is_submenu_open(1, "png"));
	}
}
