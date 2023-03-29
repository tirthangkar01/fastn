pub struct HtmlUI {
    pub html: String,
    pub dependencies: String,
    pub variables: String,
    pub functions: String,
    pub variable_dependencies: String,
    pub outer_events: String,
    pub dummy_html: String,
    pub raw_html: String,
    pub mutable_variable: String,
    pub immutable_variable: String,
    pub html_data: HTMLData,
    pub js: String,
    pub css: String,
}

pub struct HTMLData {
    pub title: Option<String>,
    pub og_title: Option<String>,
}

impl ftd::node::HTMLData {
    fn to_html_data(&self) -> HTMLData {
        HTMLData {
            title: self.title.value.to_owned(),
            og_title: self.og_title.value.to_owned(),
        }
    }
}

impl HtmlUI {
    #[tracing::instrument(skip_all)]
    pub fn from_node_data(
        node_data: ftd::node::NodeData,
        id: &str,
        test: bool,
    ) -> ftd::html1::Result<HtmlUI> {
        use itertools::Itertools;

        let tdoc = ftd::interpreter2::TDoc::new(
            node_data.name.as_str(),
            &node_data.aliases,
            &node_data.bag,
        );

        let functions = ftd::html1::FunctionGenerator::new(id).get_functions(&node_data)?;
        let (dependencies, var_dependencies) = ftd::html1::dependencies::DependencyGenerator::new(
            id,
            &node_data.node,
            &node_data.html_data,
            &tdoc,
        )
        .get_dependencies()?;
        let variable_dependencies = ftd::html1::VariableDependencyGenerator::new(id, &tdoc)
            .get_set_functions(&var_dependencies, test)?;
        let variables = ftd::html1::data::DataGenerator::new(&tdoc).get_data()?;
        let (html, outer_events, mutable_variable, immutable_variable) =
            HtmlGenerator::new(id, &tdoc).as_html_and_outer_events(node_data.node)?;
        let dummy_html = ftd::html1::DummyHtmlGenerator::new(id, &tdoc)
            .as_string_from_dummy_nodes(&node_data.dummy_nodes);

        let raw_html = ftd::html1::HelperHtmlGenerator::new(id, &tdoc)
            .as_string_from_raw_nodes(&node_data.raw_nodes);

        /*for (dependency, raw_node) in node_data.raw_nodes {
            let raw_html = RawHtmlGenerator::from_node(id, &tdoc, raw_node.node);
            dbg!("raw_nodes", &dependency, &raw_html);
        }*/

        Ok(HtmlUI {
            html,
            dependencies,
            variables: serde_json::to_string_pretty(&variables)
                .expect("failed to convert document to json"),
            functions,
            variable_dependencies,
            outer_events,
            dummy_html,
            raw_html,
            mutable_variable,
            immutable_variable,
            html_data: node_data.html_data.to_html_data(),
            js: ftd::html1::utils::get_js_html(node_data.js.into_iter().collect_vec().as_slice()),
            css: ftd::html1::utils::get_css_html(
                node_data.css.into_iter().collect_vec().as_slice(),
            ),
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct RawHtmlGenerator {
    pub name: String,
    pub html: String,
    pub properties: Vec<(String, ftd::interpreter2::Property)>,
    pub properties_string: Option<String>,
    pub iteration: Option<ftd::interpreter2::Loop>,
    pub helper_html: ftd::Map<RawHtmlGenerator>,
    pub children: Vec<RawHtmlGenerator>,
}

impl RawHtmlGenerator {
    pub(crate) fn from_node(
        id: &str,
        doc: &ftd::interpreter2::TDoc,
        node: ftd::node::Node,
    ) -> RawHtmlGenerator {
        let mut dummy_html = Default::default();
        //TODO: Remove result
        HtmlGenerator::new(id, doc)
            .as_dummy_html(node, &mut dummy_html)
            .unwrap();
        dummy_html
    }
}

pub(crate) struct HtmlGenerator<'a> {
    pub id: String,
    pub doc: &'a ftd::interpreter2::TDoc<'a>,
    pub mutable_variable: Vec<String>,
    pub immutable_variable: Vec<String>,
}

impl<'a> HtmlGenerator<'a> {
    pub fn new(id: &str, doc: &'a ftd::interpreter2::TDoc<'a>) -> HtmlGenerator<'a> {
        HtmlGenerator {
            id: id.to_string(),
            doc,
            mutable_variable: vec![],
            immutable_variable: vec![],
        }
    }

    pub fn as_dummy_html(
        &mut self,
        node: ftd::node::Node,
        dummy_html: &mut RawHtmlGenerator,
    ) -> ftd::html1::Result<()> {
        if let Some(raw_data) = node.raw_data {
            dummy_html.iteration = raw_data.iteration;
            dummy_html.properties_string = ftd::html1::utils::to_properties_string(
                self.id.as_str(),
                raw_data.properties.as_slice(),
                self.doc,
                node.node.as_str(),
            );
            dummy_html.properties = raw_data.properties;
            dummy_html.html = format!("{{{}}}", node.node);
            dummy_html.name = node.node.to_string();
            for child in node.children {
                let mut child_dummy_html = Default::default();
                self.as_dummy_html(child, &mut child_dummy_html)?;
                dummy_html.children.push(child_dummy_html);
            }
        } else {
            let data = self.as_dummy_html_(node, dummy_html)?;
            dummy_html.html = data.0;
        }

        Ok(())
    }

    pub fn as_html_and_outer_events(
        &mut self,
        node: ftd::node::Node,
    ) -> ftd::html1::Result<(String, String, String, String)> {
        let (html, events) = self.as_html_(node)?;

        let mutable_value =
            ftd::html1::utils::mutable_value(self.mutable_variable.as_slice(), self.id.as_str());
        let immutable_value = ftd::html1::utils::immutable_value(
            self.immutable_variable.as_slice(),
            self.id.as_str(),
        );

        Ok((
            html,
            ftd::html1::utils::events_to_string(events),
            mutable_value,
            immutable_value,
        ))
    }

    #[allow(clippy::type_complexity)]
    pub fn as_dummy_html_(
        &mut self,
        node: ftd::node::Node,
        dummy_html: &mut RawHtmlGenerator,
    ) -> ftd::html1::Result<(String, Vec<(String, String, String)>)> {
        if node.is_null() {
            return Ok(("".to_string(), vec![]));
        }

        if let Some(raw_data) = node.raw_data {
            let number = ftd::html1::utils::get_new_number(
                &dummy_html
                    .helper_html
                    .iter()
                    .map(|v| v.0.to_string())
                    .collect(),
                node.node.as_str(),
            );
            let node_name = format!("{}_{}", node.node, number);
            dummy_html
                .helper_html
                .insert(node_name.to_string(), Default::default());
            let helper_dummy_html = dummy_html.helper_html.get_mut(node_name.as_str()).unwrap();
            helper_dummy_html.iteration = raw_data.iteration;
            helper_dummy_html.properties_string = ftd::html1::utils::to_properties_string(
                self.id.as_str(),
                raw_data.properties.as_slice(),
                self.doc,
                node_name.as_str(),
            );
            helper_dummy_html.properties = raw_data.properties;
            helper_dummy_html.html = format!("{{{}}}", node_name);
            helper_dummy_html.name = node_name.to_string();
            for child in node.children {
                let mut child_dummy_html = Default::default();
                self.as_dummy_html(child, &mut child_dummy_html)?;
                dummy_html.children.push(child_dummy_html);
            }
            return Ok((node_name.to_string(), vec![]));
        }

        let style = format!(
            "style=\"{}\"",
            self.style_to_html(&node, /*self.visible*/ true)
        );
        let classes = self.class_to_html(&node);

        let mut outer_events = vec![];
        let attrs = {
            let mut attr = self.attrs_to_html(&node);
            let events = self.group_by_js_event(&node.events)?;
            for (name, actions) in events {
                if name.eq("onclickoutside") || name.starts_with("onglobalkey") {
                    let event = format!(
                        "window.ftd.handle_event(event, '{}', '{}', this)",
                        self.id, actions
                    );
                    outer_events.push((
                        ftd::html1::utils::full_data_id(self.id.as_str(), node.data_id.as_str()),
                        name,
                        event,
                    ));
                } else {
                    let event = format!(
                        "window.ftd.handle_event(event, '{}', '{}', this)",
                        self.id,
                        actions.replace('\"', "&quot;")
                    );
                    attr.push(' ');
                    attr.push_str(&format!("{}={}", name, quote(&event)));
                }
            }
            attr
        };

        let body = match node.text.value.as_ref() {
            Some(v) => v.to_string(),
            None => node
                .children
                .into_iter()
                .map(|v| match self.as_html_(v) {
                    Ok((html, events)) => {
                        outer_events.extend(events);
                        Ok(html)
                    }
                    Err(e) => Err(e),
                })
                .collect::<ftd::html1::Result<Vec<String>>>()?
                .join(""),
        };

        Ok((
            format!(
                "<{node} {attrs} {style} {classes}>{body}</{node}>",
                node = node.node.as_str(),
                attrs = attrs,
                style = style,
                classes = classes,
                body = body,
            ),
            outer_events,
        ))
    }

    #[allow(clippy::type_complexity)]
    pub fn as_html_(
        &mut self,
        node: ftd::node::Node,
    ) -> ftd::html1::Result<(String, Vec<(String, String, String)>)> {
        if node.is_null() {
            return Ok(("".to_string(), vec![]));
        }

        let style = format!(
            "style=\"{}\"",
            self.style_to_html(&node, /*self.visible*/ true)
        );
        let classes = self.class_to_html(&node);

        let mut outer_events = vec![];
        let attrs = {
            let mut attr = self.attrs_to_html(&node);
            let events = self.group_by_js_event(&node.events)?;
            for (name, actions) in events {
                if name.eq("onclickoutside") || name.starts_with("onglobalkey") {
                    let event = format!(
                        "window.ftd.handle_event(event, '{}', '{}', this)",
                        self.id, actions
                    );
                    outer_events.push((
                        ftd::html1::utils::full_data_id(self.id.as_str(), node.data_id.as_str()),
                        name,
                        event,
                    ));
                } else {
                    let event = format!(
                        "window.ftd.handle_event(event, '{}', '{}', this)",
                        self.id,
                        actions.replace('\"', "&quot;")
                    );
                    attr.push(' ');
                    attr.push_str(&format!("{}={}", name, quote(&event)));
                }
            }
            attr
        };

        let body = match node.text.value.as_ref() {
            Some(v) => v.to_string(),
            None => node
                .children
                .into_iter()
                .map(|v| match self.as_html_(v) {
                    Ok((html, events)) => {
                        outer_events.extend(events);
                        Ok(html)
                    }
                    Err(e) => Err(e),
                })
                .collect::<ftd::html1::Result<Vec<String>>>()?
                .join(""),
        };

        Ok((
            format!(
                "<{node} {attrs} {style} {classes}>{body}</{node}>",
                node = node.node.as_str(),
                attrs = attrs,
                style = style,
                classes = classes,
                body = body,
            ),
            outer_events,
        ))
    }

    pub fn style_to_html(&self, node: &ftd::node::Node, visible: bool) -> String {
        let mut styles: ftd::Map<String> = node
            .style
            .clone()
            .into_iter()
            .filter_map(|(k, v)| {
                v.value
                    .map(|v| match v.as_str() {
                        ftd::interpreter2::FTD_NO_VALUE => (k, "".to_string()),
                        _ => (k, v),
                    })
                    .filter(|s| !s.1.eq(ftd::interpreter2::FTD_IGNORE_KEY))
            })
            .collect();
        if !visible {
            styles.insert("display".to_string(), "none".to_string());
        }
        styles
            .iter()
            .map(|(k, v)| format!("{}: {}", *k, escape(v))) // TODO: escape needed?
            .collect::<Vec<String>>()
            .join("; ")
    }

    pub fn class_to_html(&self, node: &ftd::node::Node) -> String {
        if node.classes.is_empty() {
            return "".to_string();
        }
        format!(
            "class=\"{}\"",
            node.classes
                .iter()
                .map(|k| k.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        )
    }

    fn attrs_to_html(&mut self, node: &ftd::node::Node) -> String {
        let mut attrs = node
            .attrs
            .iter()
            .filter_map(|(k, v)| {
                if k.eq("class") {
                    return None;
                }
                v.value.as_ref().map(|v| {
                    if v.eq(ftd::interpreter2::FTD_IGNORE_KEY) {
                        return s("");
                    }
                    if v.eq(ftd::interpreter2::FTD_NO_VALUE) {
                        return s(k);
                    }
                    let v = if k.eq("data-id") {
                        ftd::html1::utils::full_data_id(self.id.as_str(), v)
                    } else {
                        v.to_string()
                    };
                    format!("{}={}", *k, quote(v.as_str()))
                })
            }) // TODO: escape needed?
            .collect::<Vec<String>>();

        if let Some(ref web_component) = node.web_component {
            for (key, val) in &web_component.properties {
                if let Some(reference) = val.reference_name() {
                    let function = if val.is_mutable() {
                        self.mutable_variable.push(reference.to_string());
                        "mutable_value"
                    } else {
                        self.immutable_variable.push(reference.to_string());
                        "immutable_value"
                    };
                    attrs.push(format!(
                        "{}=\"window.ftd.{}_{}['{}']\"",
                        key, function, self.id, reference
                    ));
                }
            }
        }

        attrs.join(" ")
    }
}

fn s(s: &str) -> String {
    s.to_string()
}

pub fn escape(s: &str) -> String {
    let s = s.replace('>', "\\u003E");
    let s = s.replace('<', "\\u003C");
    s.replace('&', "\\u0026")
}

fn quote(i: &str) -> String {
    format!("{:?}", i)
}