use failure::ResultExt;

pub fn connect(url: &unijudge::TaskUrl) -> evscode::R<Box<dyn unijudge::Session>> {
	let (username, password) = {
		let _status = crate::STATUS.push("Remembering passwords");
		crate::auth::site_credentials(&url.site)?
	};
	let sess = {
		let _status = crate::STATUS.push("Logging in");
		unijudge::connect_login(&url.site, &username, &password).compat()?
	};
	Ok(sess)
}