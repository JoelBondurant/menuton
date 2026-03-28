use iced::Event;
use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::text::{self, Paragraph};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{
	Background, Border, Color, Element, Length, Pixels, Point, Rectangle, Shadow, Size, Vector,
};

use crate::colors::{BG_PRIMARY, BORDER_PRIMARY, SHADOW_PRIMARY, TEXT_PRIMARY, TEXT_SECONDARY};
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
	Renderer: text::Renderer,
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
		_tree: &Tree,
		renderer: &mut Renderer,
		_theme: &Theme,
		_style: &renderer::Style,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		viewport: &Rectangle,
	) {
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
						let hovered = cursor.is_over(item.bounds);

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
								">",
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
		_tree: &mut Tree,
		event: &Event,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		renderer: &Renderer,
		_clipboard: &mut dyn Clipboard,
		shell: &mut Shell<'_, MenuMessage>,
		_viewport: &Rectangle,
	) {
		let geometry = MenuGeometry::new(self.roots, self.state, renderer, layout.bounds().width)
			.with_origin(Point::new(layout.bounds().x, layout.bounds().y));

		match event {
			Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				if let Some(hit) = geometry.hit_test(cursor) {
					match hit {
						Hit::Root(root) if self.state.open_root().is_some() => {
							shell.publish(MenuMessage::OpenRoot(root.id));
							shell.capture_event();
							return;
						}
						Hit::Root(_) => {
							shell.capture_event();
							return;
						}
						Hit::PanelItem(item) => match item.kind {
							ItemKind::Submenu { id, .. } => {
								shell.publish(MenuMessage::OpenSubmenu {
									depth: item.depth,
									id,
								});
								shell.capture_event();
								return;
							}
							ItemKind::Action { .. } => {
								shell.publish(MenuMessage::TrimPath(item.depth));
								shell.capture_event();
								return;
							}
							ItemKind::Separator => {
								shell.capture_event();
								return;
							}
						},
						Hit::Panel => {
							shell.capture_event();
							return;
						}
					}
				} else if self.state.open_root().is_some() && cursor.is_over(layout.bounds()) {
					shell.publish(MenuMessage::TrimPath(0));
					shell.capture_event();
					return;
				}
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if let Some(hit) = geometry.hit_test(cursor) {
					match hit {
						Hit::Root(root) => {
							shell.publish(MenuMessage::ToggleRoot(root.id));
							shell.capture_event();
							return;
						}
						Hit::PanelItem(item) => match item.kind {
							ItemKind::Action { id, .. } => {
								shell.publish(MenuMessage::Invoke(id));
								shell.capture_event();
								return;
							}
							ItemKind::Submenu { id, .. } => {
								shell.publish(MenuMessage::OpenSubmenu {
									depth: item.depth,
									id,
								});
								shell.capture_event();
								return;
							}
							ItemKind::Separator => {
								shell.capture_event();
								return;
							}
						},
						Hit::Panel => {
							shell.capture_event();
							return;
						}
					}
				} else if self.state.open_root().is_some() {
					shell.publish(MenuMessage::Close);
					shell.capture_event();
					return;
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
		tree::Tag::stateless()
	}
}

impl<'a, Theme, Renderer> From<MenuBar<'a>> for Element<'a, MenuMessage, Theme, Renderer>
where
	Theme: 'a,
	Renderer: text::Renderer + 'a,
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
	fn new<Renderer: text::Renderer>(
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

		if let Some(root_id) = state.open_root() {
			if let Some((root_index, root)) = roots
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

fn layout_panel<'a, Renderer: text::Renderer>(
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

fn draw_label<Renderer: text::Renderer>(
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
			font: renderer.default_font(),
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

fn measure_label<Renderer: text::Renderer>(
	_renderer: &Renderer,
	label: &str,
	font: Renderer::Font,
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
		font,
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
