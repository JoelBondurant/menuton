#[derive(Debug, Default)]
pub struct MenuState {
	open_root: Option<&'static str>,
	open_path: Vec<&'static str>,
}

impl MenuState {
	pub fn is_root_open(&self, id: &'static str) -> bool {
		self.open_root == Some(id)
	}

	pub fn is_submenu_open(&self, depth: usize, id: &'static str) -> bool {
		self.open_path.get(depth) == Some(&id)
	}

	pub fn open_root(&self) -> Option<&'static str> {
		self.open_root
	}

	pub fn open_path(&self) -> &[&'static str] {
		&self.open_path
	}

	pub fn set_open_root(&mut self, id: &'static str) {
		self.open_root = Some(id);
		self.open_path.clear();
	}

	pub fn set_open_submenu(&mut self, depth: usize, id: &'static str) {
		self.open_path.truncate(depth);
		self.open_path.push(id);
	}

	pub fn trim_path(&mut self, depth: usize) {
		self.open_path.truncate(depth);
	}

	pub fn close(&mut self) {
		self.open_root = None;
		self.open_path.clear();
	}
}
