pub struct FunctionGenerator {
    id: String,
}

impl FunctionGenerator {
    pub fn new(id: &str) -> FunctionGenerator {
        FunctionGenerator { id: id.to_string() }
    }

    pub fn get_functions(&self, node_data: &ftd::node::NodeData) -> String {
        let mut vector = vec![];
        for function in node_data
            .bag
            .values()
            .filter_map(|v| v.to_owned().function(node_data.name.as_str(), 0).ok())
        {
            vector.push(self.get_function(function))
        }

        vector.join("\n\n")
    }

    pub fn get_function(&self, function: ftd::interpreter2::Function) -> String {
        use itertools::Itertools;

        /*let node = dbg!(evalexpr::build_operator_tree(
            "a = a+b+f(a, b)+(j, k) + (a+b + g(a+j, k)); a"
        )
        .unwrap()); //Todo: remove unwrap
        dbg!(to_string(&node, true, &[]).as_str(),);*/

        let mut result = vec![];
        let arguments = function
            .arguments
            .iter()
            .map(|v| v.name.to_string())
            .collect_vec();
        for expression in function.expression {
            let node = evalexpr::build_operator_tree(expression.expression.as_str()).unwrap(); //Todo: remove unwrap
            result.push(ftd::html1::utils::trim_brackets(
                ExpressionGenerator
                    .to_string(&node, true, arguments.as_slice())
                    .as_str(),
            ));
        }
        let expressions = result.join("\n");
        let function_name = ftd::html1::utils::function_name_to_js_function(
            ftd::html1::utils::name_with_id(function.name.as_str(), self.id.as_str()).as_str(),
        );

        format!(
            indoc::indoc! {"
                    function {function_name}({arguments}){{
                        {expressions}
                    }}

                "},
            function_name = function_name,
            arguments = arguments.join(","),
            expressions = expressions
        )
    }
}

struct ExpressionGenerator;

impl ExpressionGenerator {
    pub fn to_string(&self, node: &evalexpr::Node, root: bool, arguments: &[String]) -> String {
        use itertools::Itertools;

        if self.is_root(node.operator()) {
            let result = node
                .children()
                .iter()
                .map(|children| self.to_string(children, false, arguments))
                .collect_vec();
            let (is_assignment_or_chain, only_one_child) =
                node.children().first().map_or((false, true), |first| {
                    /*has_operator(dbg!(&first.operator())).is_none()*/
                    let is_assignment_or_chain =
                        self.is_assignment(first.operator()) || self.is_chain(first.operator());
                    (
                        is_assignment_or_chain,
                        is_assignment_or_chain
                            || self.has_value(first.operator()).is_some()
                            || self.is_tuple(first.operator()),
                    )
                });
            let f = if !only_one_child {
                format!("({})", result.join(""))
            } else {
                result.join("")
            };

            return if root && !is_assignment_or_chain && !f.is_empty() {
                format!("return {};", f)
            } else {
                f
            };
        }

        if self.is_chain(node.operator()) {
            let mut result = vec![];
            for children in node.children() {
                let val = ftd::html1::utils::trim_brackets(
                    self.to_string(children, true, arguments).trim(),
                );
                if !val.trim().is_empty() {
                    result.push(format!(
                        "{}{}",
                        val,
                        if val.ends_with(';') { "" } else { ";" }
                    ));
                }
            }
            return result.join("\n");
        }

        if self.is_tuple(node.operator()) {
            let mut result = vec![];
            for children in node.children() {
                result.push(self.to_string(children, false, arguments));
            }
            return format!("({})", result.join(","));
        }

        if self.is_assignment(node.operator()) {
            // Todo: if node.children().len() != 2 {throw error}
            let first = node.children().first().unwrap(); //todo remove unwrap()
            let second = node.children().get(1).unwrap(); //todo remove unwrap()
            let prefix = if !arguments.contains(&first.to_string()) {
                "let "
            } else {
                ""
            };
            return vec![
                prefix.to_string(),
                self.to_string(first, false, arguments),
                node.operator().to_string(),
                self.to_string(second, false, arguments),
            ]
            .join("");
        }

        if let Some(operator) = self.has_operator(node.operator()) {
            // Todo: if node.children().len() != 2 {throw error}
            let first = node.children().first().unwrap(); //todo remove unwrap()
            let second = node.children().get(1).unwrap(); //todo remove unwrap()
            return vec![
                self.to_string(first, false, arguments),
                operator,
                self.to_string(second, false, arguments),
            ]
            .join("");
        }

        if let Some(operator) = self.has_function(node.operator()) {
            let mut result = vec![];
            for children in node.children() {
                result.push(self.to_string(children, false, arguments));
            }
            return format!("{}{}", operator.trim(), result.join(" "));
        }

        let value = node.operator().to_string();
        format!(
            "{}{}",
            value,
            if arguments.contains(&value) {
                ".value"
            } else {
                ""
            }
        )
    }

    pub fn has_value(&self, operator: &evalexpr::Operator) -> Option<String> {
        match operator {
            evalexpr::Operator::Const { .. }
            | evalexpr::Operator::VariableIdentifierRead { .. }
            | evalexpr::Operator::VariableIdentifierWrite { .. } => Some(operator.to_string()),
            _ => None,
        }
    }

    pub fn has_function(&self, operator: &evalexpr::Operator) -> Option<String> {
        match operator {
            evalexpr::Operator::FunctionIdentifier { .. } => Some(operator.to_string()),
            _ => None,
        }
    }

    pub fn is_assignment(&self, operator: &evalexpr::Operator) -> bool {
        matches!(operator, evalexpr::Operator::Assign)
    }

    pub fn is_chain(&self, operator: &evalexpr::Operator) -> bool {
        matches!(operator, evalexpr::Operator::Chain)
    }

    pub fn is_tuple(&self, operator: &evalexpr::Operator) -> bool {
        matches!(operator, evalexpr::Operator::Tuple)
    }

    pub fn has_operator(&self, operator: &evalexpr::Operator) -> Option<String> {
        if self.has_value(operator).is_none()
            && self.has_function(operator).is_none()
            && !self.is_chain(operator)
            && !self.is_root(operator)
            && !self.is_tuple(operator)
            && !self.is_assignment(operator)
        {
            Some(operator.to_string())
        } else {
            None
        }
    }

    pub fn is_root(&self, operator: &evalexpr::Operator) -> bool {
        matches!(operator, evalexpr::Operator::RootNode)
    }
}