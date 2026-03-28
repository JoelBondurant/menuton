use iced::widget::{Space, column, container, row, stack, text};
use iced::{Alignment, Element, Length, Task, Theme};

use crate::fonts::{DEJAVU_SANS_MONO, MENU_FONT};
use crate::menu::{MenuBar, MenuItem, MenuMessage, MenuRoot, MenuState};

const FILE_MENU: &[MenuItem] = &[
	MenuItem::Action {
		id: "file.new",
		label: "New",
	},
	MenuItem::Action {
		id: "file.open",
		label: "Open...",
	},
	MenuItem::Separator,
	MenuItem::Submenu {
		id: "file.export",
		label: "Export",
		items: &[
			MenuItem::Action {
				id: "file.export.png",
				label: "PNG",
			},
			MenuItem::Action {
				id: "file.export.svg",
				label: "SVG",
			},
			MenuItem::Action {
				id: "file.export.pdf",
				label: "PDF",
			},
		],
	},
	MenuItem::Separator,
	MenuItem::Action {
		id: "file.quit",
		label: "Quit",
	},
];

const EDIT_MENU: &[MenuItem] = &[
	MenuItem::Action {
		id: "edit.undo",
		label: "Undo",
	},
	MenuItem::Action {
		id: "edit.redo",
		label: "Redo",
	},
	MenuItem::Separator,
	MenuItem::Action {
		id: "edit.cut",
		label: "Cut",
	},
	MenuItem::Action {
		id: "edit.copy",
		label: "Copy",
	},
	MenuItem::Action {
		id: "edit.paste",
		label: "Paste",
	},
];

const VIEW_MENU: &[MenuItem] = &[
	MenuItem::Action {
		id: "view.zoom_in",
		label: "Zoom In",
	},
	MenuItem::Action {
		id: "view.zoom_out",
		label: "Zoom Out",
	},
	MenuItem::Separator,
	MenuItem::Submenu {
		id: "view.panels",
		label: "Panels",
		items: &[
			MenuItem::Action {
				id: "view.panels.layers",
				label: "Layers",
			},
			MenuItem::Action {
				id: "view.panels.inspector",
				label: "Inspector",
			},
			MenuItem::Action {
				id: "view.panels.console",
				label: "Console",
			},
		],
	},
];

const MENUS: &[MenuRoot] = &[
	MenuRoot {
		id: "file",
		label: "File",
		items: FILE_MENU,
	},
	MenuRoot {
		id: "edit",
		label: "Edit",
		items: EDIT_MENU,
	},
	MenuRoot {
		id: "view",
		label: "View",
		items: VIEW_MENU,
	},
];

const MENU_BAR_SPACE: f32 = 40.0;

#[derive(Debug, Clone)]
enum Message {
	Menu(MenuMessage),
}

#[derive(Default)]
struct Demo {
	menu_state: MenuState,
	last_action: Option<&'static str>,
}

pub fn run() -> iced::Result {
	iced::application(|| (Demo::default(), Task::none()), update, view)
		.title("menuton")
		.antialiasing(true)
		.font(DEJAVU_SANS_MONO)
		.default_font(MENU_FONT)
		.theme(theme)
		.run()
}

fn update(demo: &mut Demo, message: Message) -> Task<Message> {
	match message {
		Message::Menu(menu_message) => {
			if let Some(action) = demo.menu_state.update(menu_message) {
				demo.last_action = Some(action);
			}
		}
	}

	Task::none()
}

fn view(demo: &Demo) -> Element<'_, Message> {
	let menu: Element<'_, MenuMessage> = MenuBar::new(MENUS, &demo.menu_state).into();
	let menu = menu.map(Message::Menu);

	let status = text(match demo.last_action {
		Some(action) => format!("Last action: {action}"),
		None => String::from("Select a menu item to exercise the interaction model."),
	});

	let canvas = container(
		column![
			text("Demo surface"),
			text("The menu bar is now a custom widget with direct hit-testing and drawing."),
			text(
				"This keeps the demo focused on menu behavior instead of inheriting button semantics."
			)
		]
		.spacing(8),
	)
	.padding(24)
	.width(Length::Fill)
	.height(Length::Fill);

	let content = container(
		column![
			Space::new().height(MENU_BAR_SPACE),
			row![status].padding(16),
			canvas
		]
		.spacing(12)
		.align_x(Alignment::Start),
	)
	.padding(16)
	.width(Length::Fill)
	.height(Length::Fill);

	stack![content, menu]
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

fn theme(_demo: &Demo) -> Theme {
	Theme::TokyoNight
}
