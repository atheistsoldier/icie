use unijudge::{
	debris::{self, Context, Find}, reqwest::{
		self, cookie_store::Cookie, header::{ORIGIN, REFERER}, multipart, Url
	}, Error, Language, RejectionCause, Result, Submission, TaskDetails, Verdict
};

pub struct SPOJ;

impl unijudge::Backend for SPOJ {
	type CachedAuth = [Cookie<'static>; 3];
	type Session = reqwest::Client;
	type Task = String;

	fn accepted_domains(&self) -> &[&str] {
		&["www.spoj.com"]
	}

	fn deconstruct_task(&self, _domain: &str, segments: &[&str]) -> Result<Self::Task> {
		match segments {
			["problems", task] => Ok((*task).to_owned()),
			_ => Err(Error::WrongTaskUrl),
		}
	}

	fn connect(&self, client: reqwest::Client, _domain: &str) -> Self::Session {
		client
	}

	fn login(&self, session: &Self::Session, username: &str, password: &str) -> Result<()> {
		let mut resp = session
			.post("https://www.spoj.com/login/")
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/")
			.form(&[("next_raw", "/"), ("autologin", "1"), ("login_user", username), ("password", password)])
			.send()?;
		let doc = debris::Document::new(&resp.text()?);
		if resp.url().as_str() == "https://www.spoj.com/login/" {
			Err(Error::WrongCredentials)
		} else if resp.url().as_str() == "https://www.spoj.com/" {
			Ok(())
		} else {
			Err(Error::UnexpectedHTML(doc.error("unrecognized login outcome")))
		}
	}

	fn restore_auth(&self, session: &Self::Session, auth: Self::CachedAuth) -> Result<()> {
		let url = "https://www.spoj.com/".parse().unwrap();
		let [c1, c2, c3] = auth;
		let mut cookies = session.cookies().write().unwrap();
		cookies.0.insert(c1, &url).unwrap();
		cookies.0.insert(c2, &url).unwrap();
		cookies.0.insert(c3, &url).unwrap();
		Ok(())
	}

	fn cache_auth(&self, session: &Self::Session) -> Result<Option<Self::CachedAuth>> {
		let cookies = session.cookies().read().unwrap();
		let spoj = match cookies.0.get("spoj.com", "/", "SPOJ") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		let login = match cookies.0.get("spoj.com", "/", "autologin_login") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		let hash = match cookies.0.get("spoj.com", "/", "autologin_hash") {
			Some(c) => c.clone().into_owned(),
			None => return Ok(None),
		};
		Ok(Some([spoj, login, hash]))
	}

	fn task_details(&self, session: &Self::Session, task: &Self::Task) -> Result<TaskDetails> {
		let url: Url = format!("https://www.spoj.com/problems/{}/", task).parse().unwrap();
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		let title = doc.find(".breadcrumb > .active")?.text().string();
		Ok(TaskDetails { symbol: task.to_owned(), title, contest_id: "problems".to_owned(), site_short: "spoj".to_owned(), examples: None })
	}

	fn task_languages(&self, session: &Self::Session, task: &Self::Task) -> Result<Vec<Language>> {
		let url: Url = format!("https://www.spoj.com/submit/{}/", task).parse().unwrap();
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		doc.find_all("#lang > option")
			.map(|node| Ok(Language { id: node.attr("value")?.string(), name: node.text().string() }))
			.collect::<Result<_>>()
	}

	fn task_submissions(&self, session: &Self::Session, _task: &Self::Task) -> Result<Vec<Submission>> {
		let user = session.cookies().read().unwrap().0.get("spoj.com", "/", "autologin_login").ok_or(Error::AccessDenied)?.value().to_owned();
		let url: Url = format!("https://www.spoj.com/status/{}/", user).parse().unwrap();
		let mut resp = session.get(url).send()?;
		let doc = debris::Document::new(&resp.text()?);
		Ok(doc
			.find_all("table.newstatus > tbody > tr")
			.map(|row| {
				Ok(unijudge::Submission {
					id: row.child(1)?.text().string(),
					verdict: row.find(".statusres")?.text().map(|text| {
						let part = &text[..text.find("\n").unwrap_or(text.len())];
						match part {
							"accepted" => Ok(Verdict::Accepted),
							"wrong answer" => Ok(Verdict::Rejected { cause: Some(RejectionCause::WrongAnswer), test: None }),
							"time limit exceeded" => Ok(Verdict::Rejected { cause: Some(RejectionCause::TimeLimitExceeded), test: None }),
							"compilation error" => Ok(Verdict::Rejected { cause: Some(RejectionCause::CompilationError), test: None }),
							"runtime error    (SIGFPE)" | "runtime error    (SIGSEGV)" | "runtime error    (SIGABRT)" | "runtime error    (NZEC)" => {
								Ok(Verdict::Rejected { cause: Some(RejectionCause::RuntimeError), test: None })
							},
							"internal error" => Ok(Verdict::Rejected { cause: Some(RejectionCause::SystemError), test: None }),
							"waiting.." => Ok(Verdict::Pending { test: None }),
							"compiling.." => Ok(Verdict::Pending { test: None }),
							"running judge.." => Ok(Verdict::Pending { test: None }),
							"running.." => Ok(Verdict::Pending { test: None }),
							_ => part
								.parse::<f64>()
								.map(|score| Verdict::Scored { score, max: None, cause: None, test: None })
								.map_err(|_| Err::<Verdict, String>(format!("unrecognized SPOJ verdict {:?}", part))),
						}
					})?,
				})
			})
			.collect::<Result<_>>()?)
	}

	fn task_submit(&self, session: &Self::Session, task: &Self::Task, language: &Language, code: &str) -> Result<String> {
		let mut resp = session
			.post("https://www.spoj.com/submit/complete/")
			.multipart(
				multipart::Form::new()
					.part("subm_file", multipart::Part::bytes(Vec::new()).file_name("").mime_str("application/octet-stream").unwrap())
					.text("file", code.to_owned())
					.text("lang", language.id.to_owned())
					.text("problemcode", task.to_owned())
					.text("submit", "Submit!"),
			)
			.header(ORIGIN, "https://www.spoj.com")
			.header(REFERER, "https://www.spoj.com/submit/TEST/")
			.send()?;
		let doc = unijudge::debris::Document::new(&resp.text()?);
		if doc.find("title")?.text().string().contains("Authorisation required") {
			return Err(Error::AccessDenied);
		}
		Ok(doc.find("#content > input")?.attr("value")?.string())
	}
}
