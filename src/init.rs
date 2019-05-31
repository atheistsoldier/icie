use crate::{dir, util};
use evscode::{E, R};
use std::path::{Path, PathBuf};

#[evscode::config(
	description = "The name of the code template used for initializing new projects. The list of code templates' names and paths can be found under the icie.template.list \
	               configuration entry."
)]
static SOLUTION_TEMPLATE: evscode::Config<String> = "C++";

fn init(root: &Path) -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let url = evscode::InputBox::new()
		.prompt("Enter task URL or leave empty")
		.placeholder("https://codeforces.com/contest/.../problem/...")
		.ignore_focus_out()
		.build()
		.wait()
		.map(|url| if url.trim().is_empty() { None } else { Some(url) })
		.ok_or_else(E::cancel)?;
	init_manifest(root, &url)?;
	init_template(root)?;
	init_examples(root, &url)?;
	evscode::open_folder(root, false);
	Ok(())
}

fn init_manifest(root: &Path, url: &Option<String>) -> R<()> {
	let manifest = crate::manifest::Manifest::new_project(url.clone());
	manifest.save(root)?;
	Ok(())
}
fn init_template(root: &Path) -> R<()> {
	let solution = root.join(format!("{}.{}", dir::SOLUTION_STEM.get(), dir::CPP_EXTENSION.get()));
	if !solution.exists() {
		let req_id = SOLUTION_TEMPLATE.get();
		let list = crate::template::LIST.get();
		let path = list
			.iter()
			.find(|(id, _)| **id == *req_id)
			.ok_or_else(|| {
				E::error(format!(
					"template '{}' does not exist; go to the settings(Ctrl+,), and either change the template(icie.init.solutionTemplate) or add a template with this \
					 name(icie.template.list)",
					req_id
				))
			})?
			.1;
		let tpl = crate::template::load(&path)?;
		util::fs_write(solution, tpl.code)?;
	}
	Ok(())
}
fn init_examples(root: &Path, url: &Option<String>) -> R<()> {
	if let Some(url) = url {
		let url = unijudge::TaskUrl::deconstruct(&url).map_err(util::from_unijudge_error)?;
		let sess = crate::net::connect(&url)?;
		let cont = sess.contest(&url.contest);
		let examples_dir = root.join("tests").join("example");
		util::fs_create_dir_all(&examples_dir)?;
		let tests = {
			let _status = crate::STATUS.push("Downloading tests");
			cont.examples(&url.task).map_err(util::from_unijudge_error)?
		};
		for (i, test) in tests.into_iter().enumerate() {
			util::fs_write(examples_dir.join(format!("{}.in", i + 1)), &test.input)?;
			util::fs_write(examples_dir.join(format!("{}.out", i + 1)), &test.output)?;
		}
	}
	Ok(())
}

#[evscode::command(title = "ICIE Init", key = "alt+f11")]
fn new() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root = ASK_FOR_PATH.get().query(&*dir::PROJECT_DIRECTORY.get(), &dir::random_codename())?;
	let dir = util::TransactionDir::new(&root)?;
	init(&root)?;
	dir.commit();
	Ok(())
}

#[evscode::command(title = "ICIE Init existing")]
fn existing() -> R<()> {
	let _status = crate::STATUS.push("Initializing");
	let root = evscode::workspace_root()?;
	init(&root)?;
	Ok(())
}

#[evscode::config(
	description = "By default, when initializing a project, the project directory will be created in the directory determined by icie.dir.projectDirectory configuration entry, \
	               and the name will be chosen from a selection of cute animals. This options allows to instead specify the directory every time."
)]
static ASK_FOR_PATH: evscode::Config<PathDialog> = PathDialog::None;

#[derive(Debug, evscode::Configurable)]
enum PathDialog {
	#[evscode(name = "No")]
	None,
	#[evscode(name = "With a VS Code input box")]
	InputBox,
	#[evscode(name = "With a system dialog")]
	SystemDialog,
}

impl PathDialog {
	fn query(&self, directory: &Path, codename: &str) -> R<PathBuf> {
		let basic = directory.join(codename);
		let basic_str = basic.display().to_string();
		match self {
			PathDialog::None => Ok(basic),
			PathDialog::InputBox => Ok(PathBuf::from(
				evscode::InputBox::new()
					.ignore_focus_out()
					.prompt("New project directory")
					.value(&basic_str)
					.value_selection(basic_str.len() - codename.len(), basic_str.len())
					.build()
					.wait()
					.ok_or_else(E::cancel)?,
			)),
			PathDialog::SystemDialog => Ok(evscode::OpenDialog::new().directory().action_label("Init").build().wait().ok_or_else(E::cancel)?),
		}
	}
}
