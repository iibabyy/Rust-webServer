/*---------------------------------------------------------------*/
/*-------------------------[ LOCATIONS ]-------------------------*/
/*---------------------------------------------------------------*/

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{request::Method, LocationBlock};

use super::{
    parsing,
    server::Server,
    traits::{config::Config, handler::Handler},
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Location {
    internal: bool,
    exact_path: bool,
    auto_index: bool,
    path: PathBuf,
    root: Option<PathBuf>,
    upload_folder: Option<PathBuf>,
    alias: Option<PathBuf>,
    max_body_size: Option<usize>,
    redirect: Option<String>,
    index: Option<String>,
    return_: Option<(u16, Option<String>)>,
    methods: Option<Vec<Method>>,
    error_pages: HashMap<u16, String>,
    error_redirect: HashMap<u16, (Option<u16>, String)>,
    cgi: HashMap<String, PathBuf>,
    none_cgi: bool,
    infos: HashMap<String, Vec<String>>,
    server: Option<Arc<Server>>,
}

impl Handler for Location {}

impl Config for Location {
    fn path(&self) -> &PathBuf /*---------------------------------*/ { &self.path }
    fn internal(&self) -> bool /*---------------------------------*/ { self.internal }
    fn auto_index(&self) -> bool /*-------------------------------*/ { self.auto_index }
    fn is_location(&self) -> bool /*------------------------------*/ { false }
    fn index(&self) -> Option<&String> /*-------------------------*/ { self.index.as_ref() }
    fn root(&self) -> Option<&PathBuf> /*-------------------------*/ { self.root.as_ref() }
    fn alias(&self) -> Option<&PathBuf> /*------------------------*/ { self.alias.as_ref() }
    fn methods(&self) -> Option<&Vec<Method>> /*------------------*/ { self.methods.as_ref() }
    fn max_body_size(&self) -> Option<&usize> /*------------------*/ { self.max_body_size.as_ref() }
    fn cgi(&self) -> &HashMap<String, PathBuf> /*-----------------*/ { &self.cgi }
    fn upload_folder(&self) -> Option<&PathBuf> /*----------------*/ { self.upload_folder.as_ref() }
    fn error_pages(&self) -> &HashMap<u16, String> /*-------------*/ { &self.error_pages }
    fn return_(&self) -> Option<&(u16, Option<String>)> /*--------*/ { self.return_.as_ref() }
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)> { &self.error_redirect }
    fn port(&self) -> Option<&u16> /*-----------------------------*/ { None }
    fn name(&self) -> Option<&Vec<String>> /*---------------------*/ { None }
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>> /*-*/ { None }
}

#[allow(dead_code)]
impl Location {
    pub(super) fn new(location: LocationBlock, server: &Server) -> Result<Self, String> {
        let mut new_location = Location {
            path: PathBuf::from(location.path),
            exact_path: (location.modifier == Some("=".to_owned())),
            error_pages: HashMap::new(),
            error_redirect: HashMap::new(),
            max_body_size: None,
            return_: None,
            root: None,
            upload_folder: None,
            alias: None,
            index: None,
            methods: None,
            redirect: None,
            auto_index: false,
            none_cgi: false,
            internal: false,
            infos: HashMap::new(),
            cgi: HashMap::new(),
            server: None,
        };

        for (name, infos) in location.directives {
            match name.as_str() {
                "root" => {
                    if new_location.root.is_some() { return Err(format!("invalid field: root: root cannot be set with alias")); }
  
                    let root = parsing::extract_root(infos);
                    match root {
						Ok(path) => new_location.root = Some(path),
                        Err(e) => {
							return Err(format!("location ({}) : {}", new_location.path.display(), e))
						},
                    }
                }
                "alias" => {
                    if new_location.root.is_some() {
                        return Err(format!("invalid field: alias: alias cannot be set with root"));
                    } else {
                        new_location.alias = Some(parsing::extract_alias(infos)?)
                    }
                }
                "upload_folder" => {
                    new_location.upload_folder = Some(parsing::extract_upload_folder(infos)?)
                }
                "index" => {
                    let index = parsing::extract_index(infos);
                    match index {
						Ok(index) => new_location.index = Some(index),
                        Err(e) => {
                            return Err(format!("location ({}) : {}", new_location.path.display(), e))
                        },
                    }
                }
                "auto_index" => {
                    let auto_index = parsing::extract_auto_index(infos);
                    match auto_index {
						Ok(is_true) => new_location.auto_index = is_true,
                        Err(e) => {
                            return Err(format!("location ({}) : {}", new_location.path.display(), e))
                        },
                    }
                }
                "client_max_body_size" => {
                    let max_body_size = parsing::extract_max_body_size(infos);
                    match max_body_size {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(max_size) => new_location.max_body_size = Some(max_size),
                    }
                }
                "cgi" => {
                    if infos.len() == 1 && infos[1] == "none" {
                        new_location.none_cgi = true;
                    } else {
                        let (extension, path) = match parsing::extract_cgi(infos) {
                            Err(e) => {
                                return Err(format!(
                                    "location ({}) : {}",
                                    new_location.path.display(),
                                    e
                                ))
                            }
                            Ok(cgi) => cgi,
                        };
                        new_location.cgi.insert(extension, path);
                    }
                }
                "allowed_methods" => {
                    if infos.len() < 1 {
                        return Err(format!(
                            "location ({}) : invalid field: allowed_methods",
                            new_location.path.display()
                        ));
                    }
                    if new_location.methods.is_none() {
                        new_location.methods = Some(Vec::new())
                    }

                    new_location.methods.as_mut().unwrap().append(
                        &mut infos
                            .iter()
                            .map(|method| Method::from(&method[..]))
                            .collect(),
                    );
                }
                "redirect" => {
                    if infos.len() != 1 {
                        return Err(format!(
                            "location ({}) : invalid field: redirect",
                            new_location.path.display()
                        ));
                    }
                    new_location.redirect = Some(infos[0].clone());
                }
                "return" => {
                    new_location.return_ = match parsing::extract_return(infos) {
                        Err(e) => {
                            return Err(format!(
                                "location ({}) : {}",
                                new_location.path.display(),
                                e
                            ))
                        }
                        Ok(res) => Some(res),
                    }
                }
                "internal" => {
                    new_location.internal = true;
                }
                "error_page" => {
                    let (pages, redirect) = parsing::extract_error_page(infos)?;
                    let hash = &mut new_location.error_pages;
                    if pages.is_some() {
                        pages
                            .unwrap()
                            .iter()
                            .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                            .last();
                    }
                    let hash = &mut new_location.error_redirect;
                    if redirect.is_some() {
                        redirect
                            .unwrap()
                            .iter()
                            .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                            .last();
                    }
                }
                _ => {
                    new_location.infos.insert(name, infos);
                }
            }
        }

        new_location.complete_with_server_directives(server);

        if new_location.none_cgi == true {
            new_location.cgi.clear();
        } if new_location.upload_folder.is_some() && new_location.root.is_some() {
			Self::add_root_to_upload_folder(&mut new_location);	
		}

        Ok(new_location)
    }

	fn add_root_to_upload_folder(location: &mut Location) {
		let upload_folder = location.upload_folder.as_ref().unwrap().to_string_lossy();
		let root = location.root.as_ref().unwrap().to_string_lossy();
		
		let upload_folder = PathBuf::from(
			format!("{upload_folder}/{root}")
		);

		location.upload_folder = Some(upload_folder);
	}

    fn complete_with_server_directives(&mut self, server: &Server) {
        self.internal = self.internal || server.internal();
        self.auto_index = self.auto_index || server.auto_index();

        if self.root.is_none() && server.root().is_some() {
            self.root = Some(server.root().unwrap().clone());
        }
        if self.upload_folder.is_none() && server.upload_folder().is_some() {
            self.upload_folder = Some(server.upload_folder().unwrap().clone());
        }
        if self.index.is_none() && server.index().is_some() {
            self.index = Some(server.index().unwrap().clone());
        }
        if self.max_body_size.is_none() && server.max_body_size().is_some() {
            self.max_body_size = Some(server.max_body_size().unwrap().clone());
        }
        // if self.methods.is_none() && server.methods().is_some() {
        //     self.methods = Some(server.methods().unwrap().clone());
        // }
        if self.return_.is_none() && server.return_().is_some() {
            self.return_ = Some(server.return_().unwrap().clone());
        }
		if self.cgi.is_empty() && !server.cgi().is_empty() {
            self.cgi = server.cgi().clone();
        }
        if self.error_pages.is_empty() && !server.error_pages().is_empty() {
            self.error_pages = server.error_pages().clone();
        }
        if self.error_redirect.is_empty() && !server.error_redirect().is_empty() {
            self.error_redirect = server.error_redirect().clone();
        }
    }

    pub fn add_server_ref(&mut self, serv: Arc<Server>) {
        self.server = Some(serv);
    }

    pub fn find(&self, name: String) -> Option<&Vec<String>> {
        self.infos.get(&name)
    }
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn exact_path(&self) -> bool {
        self.exact_path
    }
}
