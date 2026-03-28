use iced::Event;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::text::{self, Paragraph};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::keyboard::{self, key};
use iced::{
	Background, Border, Color, Element, Length, Pixels, Point, Rectangle, Shadow, Size, Vector,
};

use crate::colors::{BG_PRIMARY, BORDER_PRIMARY, SHADOW_PRIMARY, TEXT_PRIMARY, TEXT_SECONDARY};
use crate::fonts::MENU_FONT;
use crate::menu::{MenuItem, MenuRoot};

const BAR_HEIGHT: f32 = 32.0;
const BAR_ITEM_PADDING_X: f32 = 12.0;
const BAR_ITEM_GAP: f32 = 4.0;
const PANEL_GAP: f32 = 2.0;
const PANEL_PADDING: f32 = 6.0;
const PANEL_ITEM_HEIGHT: f32 = 28.0;
const PANEL_SEPARATOR_HEIGHT: f32 = 8.0;
const PANEL_MIN_WIDTH: f32 = 180.0;
const LABEL_SIZE: Pixels = Pixels(16.0);
const PANEL_TEXT_OFFSET: f32 = 10.0;
const ARROW_GUTTER: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuMessage {
	ToggleRoot(&'static str),
	OpenRoot(&'static str),
	OpenSubmenu { depth: usize, id: &'static str },
	TrimPath(usize),
	Invoke(&'static str),
	Close,
}

#[derive(Debug, Default)]
pub struct MenuState {
	open_root: Option<&'static str>,
	open_path: Vec<&'static str>,
}

impl MenuState {
	pub fn update(&mut self, message: MenuMessage) -> Option<&'static str> {
		match message {
			MenuMessage::ToggleRoot(id) => {
				if self.open_root == Some(id) {
					self.close();
				} else {
					self.open_root = Some(id);
					self.open_path.clear();
				}

				None
			}
			MenuMessage::OpenRoot(id) => {
				if self.open_root != Some(id) {
					self.open_root = Some(id);
					self.open_path.clear();
				}

				None
			}
			MenuMessage::OpenSubmenu { depth, id } => {
				self.open_path.truncate(depth);
				self.open_path.push(id);
				None
			}
			MenuMessage::TrimPath(depth) => {
				self.open_path.truncate(depth);
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

	pub fn is_root_open(&self, id: &'static str) -> bool {
		self.open_root == Some(id)
	}

	pub fn is_submenu_open(&self, depth: usize, id: &'static str) -> bool {
		self.open_path.get(depth) == Some(&id)
	}

	pub fn open_root(&self) -> Option<&'static str> {
		self.open_root
	}

	pub fn close(&mut self) {
		self.open_root = None;
		self.open_path.clear();
	}
}

#[derive(Debug, Default)]
struct WidgetState {
	keyboard_navigation: bool,
	focus_root: Option<&'static str>,
	focus_path: Vec<&'static str>,
}

impl WidgetState {
	fn clear(&mut self) {
		self.keyboard_navigation = false;
		self.focus_root = None;
		self.focus_path.clear();
	}

	fn sync(&mut self, roots: &[MenuRoot], menu_state: &MenuState) {
		let Some(open_root) = menu_state.open_root() else {
			self.clear();
			return;
		};

		self.focus_root = Some(open_root);

		let Some(root) = root_by_id(roots, open_root) else {
			self.clear();
			return;
		};

		let visible_depths = menu_state.open_path.len() + 1;
		self.focus_path.truncate(visible_depths);

		let mut items = root.items;

		for depth in 0..visible_depths {
			let fallback = first_selectable(items);
			let focused = self
				.focus_path
				.get(depth)
				.copied()
				.filter(|id| selectable_item(items, id).is_some())
				.or(fallback);

			let Some(focused) = focused else {
				self.focus_path.truncate(depth);
				return;
			};

			if depth < self.focus_path.len() {
				self.focus_path[depth] = focused;
			} else {
				self.focus_path.push(focused);
			}

			if let Some(submenu_id) = menu_state.open_path.get(depth) {
				let Some(next_items) = submenu_items(items, submenu_id) else {
					self.focus_path.truncate(depth + 1);
					return;
				};

				items = next_items;
			}
		}
	}

	fn focus_root_panel(&mut self, roots: &[MenuRoot], root_id: &'static str) {
		self.keyboard_navigation = true;
		self.focus_root = Some(root_id);
		self.focus_path.clear();

		if let Some(root) = root_by_id(roots, root_id)
			&& let Some(first) = first_selectable(root.items)
		{
			self.focus_path.push(first);
		}
	}

	fn focus_current_panel(
		&mut self,
		roots: &[MenuRoot],
		menu_state: &MenuState,
		direction: MoveDirection,
	) -> bool {
		self.sync(roots, menu_state);

		let Some((depth, items)) = focused_panel_items(roots, menu_state, self) else {
			return false;
		};

		let current = self.focus_path.get(depth).copied();
		let next = match (current, direction) {
			(Some(current), MoveDirection::Next) => next_selectable(items, current),
			(Some(current), MoveDirection::Previous) => previous_selectable(items, current),
			(None, _) => first_selectable(items),
		};

		let Some(next) = next else {
			return false;
		};

		self.keyboard_navigation = true;

		if depth < self.focus_path.len() {
			self.focus_path[depth] = next;
			self.focus_path.truncate(depth + 1);
		} else {
			self.focus_path.push(next);
		}

		true
	}

	fn focus_submenu(&mut self, items: &[MenuItem], depth: usize, id: &'static str) -> bool {
		let Some(children) = submenu_items(items, id) else {
			return false;
		};

		self.keyboard_navigation = true;

		if depth < self.focus_path.len() {
			self.focus_path[depth] = id;
			self.focus_path.truncate(depth + 1);
		} else {
			self.focus_path.push(id);
		}

		if let Some(first) = first_selectable(children) {
			self.focus_path.push(first);
		}

		true
	}

	fn focused(&self, id: &'static str, depth: usize) -> bool {
		self.keyboard_navigation && self.focus_path.get(depth) == Some(&id)
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
					key::Key::Named(key::Named::Escape) if self.state.open_root().is_some() => {
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
					key::Key::Named(key::Named::ArrowRight) => {
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
					key::Key::Named(key::Named::ArrowLeft) => {
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
					key::Key::Named(key::Named::Enter) => {
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

#[derive(Debug, Clone, Copy)]
struct RootGeometry {
	id: &'static str,
	label: &'static str,
	bounds: Rectangle,
}

#[derive(Debug)]
struct PanelGeometry<'a> {
	bounds: Rectangle,
	items: Vec<ItemGeometry<'a>>,
}

#[derive(Debug, Clone, Copy)]
struct ItemGeometry<'a> {
	depth: usize,
	bounds: Rectangle,
	kind: ItemKind<'a>,
}

#[derive(Debug, Clone, Copy)]
enum ItemKind<'a> {
	Action { id: &'static str, label: &'a str },
	Submenu { id: &'static str, label: &'a str },
	Separator,
}

#[derive(Debug)]
struct MenuGeometry<'a> {
	roots: Vec<RootGeometry>,
	panels: Vec<PanelGeometry<'a>>,
	bar_bounds: Rectangle,
}

impl<'a> MenuGeometry<'a> {
	fn new<Renderer: text::Renderer<Font = iced::Font>>(
		roots: &'a [MenuRoot],
		state: &'a MenuState,
		renderer: &Renderer,
		width: f32,
	) -> Self {
		let font = renderer.default_font();
		let line_height = text::LineHeight::default();

		let mut x = 0.0;
		let mut root_geometries = Vec::with_capacity(roots.len());

		for root in roots {
			let label_width = measure_label(renderer, root.label, font, line_height);
			let item_width = label_width + BAR_ITEM_PADDING_X * 2.0;

			root_geometries.push(RootGeometry {
				id: root.id,
				label: root.label,
				bounds: Rectangle {
					x,
					y: 0.0,
					width: item_width,
					height: BAR_HEIGHT,
				},
			});

			x += item_width + BAR_ITEM_GAP;
		}

		let bar_bounds = Rectangle {
			x: 0.0,
			y: 0.0,
			width,
			height: BAR_HEIGHT,
		};

		let mut panels = Vec::new();

		if let Some(root_id) = state.open_root()
			&& let Some((root_index, root)) = roots
				.iter()
				.enumerate()
				.find(|(_, root)| root.id == root_id)
			{
				let anchor = root_geometries[root_index].bounds;
				let mut current_items = root.items;
				let mut panel_x = anchor.x;
				let mut panel_y = BAR_HEIGHT + PANEL_GAP;

				for depth in 0..=state.open_path.len() {
					let panel = layout_panel(
						current_items,
						depth,
						renderer,
						font,
						line_height,
						Point::new(panel_x, panel_y),
					);

					let next_items = state.open_path.get(depth).and_then(|submenu_id| {
						panel.items.iter().find_map(|item| match item.kind {
							ItemKind::Submenu { id, .. } if id == *submenu_id => {
								let child = submenu_items(current_items, id)?;
								panel_x = item.bounds.x + item.bounds.width + PANEL_GAP;
								panel_y = item.bounds.y;
								Some(child)
							}
							_ => None,
						})
					});

					panels.push(panel);

					let Some(items) = next_items else {
						break;
					};

					current_items = items;
				}
		}

		Self {
			roots: root_geometries,
			panels,
			bar_bounds,
		}
	}

	fn with_origin(mut self, origin: Point) -> Self {
		self.bar_bounds.x += origin.x;
		self.bar_bounds.y += origin.y;

		for root in &mut self.roots {
			root.bounds.x += origin.x;
			root.bounds.y += origin.y;
		}

		for panel in &mut self.panels {
			panel.bounds.x += origin.x;
			panel.bounds.y += origin.y;

			for item in &mut panel.items {
				item.bounds.x += origin.x;
				item.bounds.y += origin.y;
			}
		}

		self
	}

	fn size(&self) -> Size {
		let width = self
			.panels
			.iter()
			.fold(self.bar_bounds.width, |right, panel| {
				right.max(panel.bounds.x + panel.bounds.width)
			});

		let height = self
			.panels
			.iter()
			.fold(self.bar_bounds.height, |bottom, panel| {
				bottom.max(panel.bounds.y + panel.bounds.height)
			});

		Size::new(width, height)
	}

	fn hit_test(&self, cursor: mouse::Cursor) -> Option<Hit<'a>> {
		for root in &self.roots {
			if cursor.is_over(root.bounds) {
				return Some(Hit::Root(*root));
			}
		}

		for panel in &self.panels {
			if cursor.is_over(panel.bounds) {
				for item in &panel.items {
					if cursor.is_over(item.bounds) {
						return Some(Hit::PanelItem(*item));
					}
				}

				return Some(Hit::Panel);
			}
		}

		None
	}
}

#[derive(Debug, Clone, Copy)]
enum Hit<'a> {
	Root(RootGeometry),
	Panel,
	PanelItem(ItemGeometry<'a>),
}

fn layout_panel<'a, Renderer: text::Renderer<Font = iced::Font>>(
	items: &'a [MenuItem],
	depth: usize,
	renderer: &Renderer,
	font: Renderer::Font,
	line_height: text::LineHeight,
	origin: Point,
) -> PanelGeometry<'a> {
	let mut width = PANEL_MIN_WIDTH;

	for item in items {
		let label = match item {
			MenuItem::Action { label, .. } | MenuItem::Submenu { label, .. } => *label,
			MenuItem::Separator => continue,
		};

		width = width.max(
			measure_label(renderer, label, font, line_height)
				+ PANEL_TEXT_OFFSET * 2.0
				+ ARROW_GUTTER,
		);
	}

	let height = items.iter().fold(PANEL_PADDING * 2.0, |height, item| {
		height
			+ match item {
				MenuItem::Separator => PANEL_SEPARATOR_HEIGHT,
				_ => PANEL_ITEM_HEIGHT,
			}
	});

	let mut y = origin.y + PANEL_PADDING;
	let mut geometries = Vec::with_capacity(items.len());

	for item in items {
		let item_height = match item {
			MenuItem::Separator => PANEL_SEPARATOR_HEIGHT,
			_ => PANEL_ITEM_HEIGHT,
		};

		let bounds = Rectangle {
			x: origin.x + PANEL_PADDING,
			y,
			width: width - PANEL_PADDING * 2.0,
			height: item_height,
		};

		let kind = match item {
			MenuItem::Action { id, label } => ItemKind::Action { id, label },
			MenuItem::Submenu { id, label, .. } => ItemKind::Submenu { id, label },
			MenuItem::Separator => ItemKind::Separator,
		};

		geometries.push(ItemGeometry {
			depth,
			bounds,
			kind,
		});

		y += item_height;
	}

	PanelGeometry {
		bounds: Rectangle {
			x: origin.x,
			y: origin.y,
			width,
			height,
		},
		items: geometries,
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

fn measure_label<Renderer: text::Renderer<Font = iced::Font>>(
	_renderer: &Renderer,
	label: &str,
	_font: Renderer::Font,
	line_height: text::LineHeight,
) -> f32 {
	let paragraph = <Renderer::Paragraph as Paragraph>::with_text(text::Text {
		content: label,
		bounds: Size::new(
			f32::INFINITY,
			f32::from(line_height.to_absolute(LABEL_SIZE)),
		),
		size: LABEL_SIZE,
		line_height,
		font: MENU_FONT,
		align_x: text::Alignment::Left,
		align_y: iced::alignment::Vertical::Center,
		shaping: text::Shaping::Basic,
		wrapping: text::Wrapping::None,
	});

	paragraph.min_width().ceil()
}

fn submenu_items<'a>(items: &'a [MenuItem], id: &str) -> Option<&'a [MenuItem]> {
	items.iter().find_map(|item| match item {
		MenuItem::Submenu {
			id: submenu_id,
			items,
			..
		} if *submenu_id == id => Some(*items),
		_ => None,
	})
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

#[derive(Debug, Clone, Copy)]
enum MoveDirection {
	Next,
	Previous,
}

fn root_by_id<'a>(roots: &'a [MenuRoot], id: &str) -> Option<&'a MenuRoot> {
	roots.iter().find(|root| root.id == id)
}

fn panel_items<'a>(
	roots: &'a [MenuRoot],
	state: &'a MenuState,
	depth: usize,
) -> Option<&'a [MenuItem]> {
	let root = root_by_id(roots, state.open_root()?)?;
	let mut items = root.items;

	for submenu_id in state.open_path.iter().take(depth) {
		items = submenu_items(items, submenu_id)?;
	}

	Some(items)
}

fn focused_panel_items<'a>(
	roots: &'a [MenuRoot],
	state: &'a MenuState,
	widget_state: &WidgetState,
) -> Option<(usize, &'a [MenuItem])> {
	let depth = widget_state.focus_path.len().checked_sub(1)?;
	panel_items(roots, state, depth).map(|items| (depth, items))
}

fn selectable_item<'a>(items: &'a [MenuItem], id: &str) -> Option<&'a MenuItem> {
	items.iter().find(|item| match item {
		MenuItem::Action { id: item_id, .. } | MenuItem::Submenu { id: item_id, .. } => {
			*item_id == id
		}
		MenuItem::Separator => false,
	})
}

fn first_selectable(items: &[MenuItem]) -> Option<&'static str> {
	items.iter().find_map(item_id)
}

fn next_selectable(items: &[MenuItem], current: &'static str) -> Option<&'static str> {
	cycle_selectable(items, current, 1)
}

fn previous_selectable(items: &[MenuItem], current: &'static str) -> Option<&'static str> {
	cycle_selectable(items, current, -1)
}

fn cycle_selectable(
	items: &[MenuItem],
	current: &'static str,
	step: isize,
) -> Option<&'static str> {
	let ids: Vec<_> = items.iter().filter_map(item_id).collect();
	let current_index = ids.iter().position(|id| *id == current)?;

	if ids.is_empty() {
		return None;
	}

	let len = ids.len() as isize;
	let next_index = (current_index as isize + step).rem_euclid(len) as usize;
	ids.get(next_index).copied()
}

fn item_id(item: &MenuItem) -> Option<&'static str> {
	match item {
		MenuItem::Action { id, .. } | MenuItem::Submenu { id, .. } => Some(*id),
		MenuItem::Separator => None,
	}
}

fn adjacent_root<'a>(
	roots: &'a [MenuRoot],
	current: &'static str,
	offset: isize,
) -> Option<&'a MenuRoot> {
	let index = roots.iter().position(|root| root.id == current)?;
	let len = roots.len() as isize;
	let next_index = (index as isize + offset).rem_euclid(len) as usize;
	roots.get(next_index)
}

fn is_menu_activation(key: &key::Key<&str>, modifiers: keyboard::Modifiers) -> bool {
	modifiers.command()
		&& modifiers.shift()
		&& !modifiers.alt()
		&& matches!(key, key::Key::Character("m" | "M"))
}

fn navigation_direction(key: &key::Key<&str>, shift: bool) -> Option<MoveDirection> {
	match key {
		key::Key::Named(key::Named::ArrowDown) => Some(MoveDirection::Next),
		key::Key::Named(key::Named::ArrowUp) => Some(MoveDirection::Previous),
		key::Key::Named(key::Named::Tab) if shift => Some(MoveDirection::Previous),
		key::Key::Named(key::Named::Tab) => Some(MoveDirection::Next),
		_ => None,
	}
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
