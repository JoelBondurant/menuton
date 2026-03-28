#[derive(Debug, Clone, Copy)]
pub struct MenuRoot {
	pub id: &'static str,
	pub label: &'static str,
	pub items: &'static [MenuItem],
}

#[derive(Debug, Clone, Copy)]
pub enum MenuItem {
	Action {
		id: &'static str,
		label: &'static str,
	},
	Submenu {
		id: &'static str,
		label: &'static str,
		items: &'static [MenuItem],
	},
	Separator,
}
