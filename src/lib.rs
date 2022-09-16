use std::{collections::HashMap, fs::{self, File}, path::{self, PathBuf}, io::Read};

use ext_php_rs::{prelude::*, types::ZendObject};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_until1},
    multi::{many0, many1},
    sequence::delimited,
    IResult,
};


#[derive(Debug)]
enum NodeType {
    Module,
    Body(String),
    Variable(String)
}


#[derive(Debug)]
struct Node {
    children: Vec<Box<Node>>,
    typ: NodeType
}

struct Environment {
    variables: HashMap<String, String>,
    parent: Option<Box<Environment>>
}



#[php_function]
pub fn hello_world(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[php_function]
pub fn read_file(path: &str) -> String {
    let path = PathBuf::from(path);
    let f_result = File::open(&path);
    let mut file = if let Ok(f) = f_result {
        f
    } else {
        return String::new();
    };
    let mut buf = String::new();
    file.read_to_string(&mut buf);
    match parse(&buf) {
        Ok(n) => format!("{:?}", n),
        Err(e) => format!("{}", e)
    }
}


fn parse(t: &str) -> IResult<&str, Node> {
    let mut n = Node {
        children: Vec::default(),
        typ: NodeType::Module,
    };
    let (_, children) = many1(parse_body)(t)?;
    n.children = children;
    Ok(("", n))
}

fn parse_body(t: &str) -> IResult<&str, Box<Node>> {
    let (rest, body) = take_until("{{")(t)?;
    Ok((rest, Box::new(Node { children: Vec::default(), typ: NodeType::Body(body.to_owned()) })))
}

fn parse_variable(t: &str) -> IResult<&str, Box<Node>> {
    let (rest, var) = tag("{{")(t)?;
    Ok((rest, Box::new(Node { children: Vec::default(), typ: NodeType::Variable(var.to_owned()) })))
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
}



#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
