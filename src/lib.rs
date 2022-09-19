use std::{collections::{HashMap, VecDeque}, fs::{self, File}, path::{self, PathBuf}, io::Read, fmt::Write};

use ext_php_rs::{prelude::*, types::{ZendObject, Zval}, boxed::ZBox};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_until1, take_till, take_while},
    multi::{many0, many1, many_till},
    sequence::{delimited, tuple},
    IResult, character::{complete::{multispace1, one_of}, is_space},
};


type NodeList = Vec<Box<dyn Renderer>>;

struct Module {
    children: NodeList
}

struct Body {
    content: String
}

struct Variable {
    name: String
}

#[derive(Debug)]
enum BlockTag {
    BlockName(String),
    Loop(Loop),
    Include(String),
    Undefined
}
struct Block {
    children: NodeList,
    tag: BlockTag
}

#[derive(Debug)]
struct Loop {
    varname: String,
    collection_name: String
}

struct Environment<'a> {
    base: &'a mut Zval,
    stack: Vec<StackFrame>
}

struct StackFrame {
    variables: Option<Zval>,
    vals: HashMap<String, String>
}



#[php_function]
pub fn hello_world(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[php_function]
pub fn read_file(path: &str, data: &mut Zval) -> String {
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
        Ok((_, n)) => {
            let mut out = String::default();
            let mut env = Environment {
                stack: Vec::default(),
                base: data
            };
            n.render(&mut out, &mut env);
            out
        },
        Err(e) => format!("{}", e)
    }
}


fn parse(t: &str) -> IResult<&str, Module> {
    let mut n = Module {
        children: Vec::default(),
    };
    let (rest, (children, _)) = many_till(parse_content, nom::combinator::eof)(t)?;
    n.children = children;
    Ok((rest, n))
}

fn parse_content(t: &str) -> IResult<&str, Box<dyn Renderer>> {

    let (rest, content) = alt((parse_variable, parse_block, parse_body))(t)?;
    Ok((rest, content))
}

fn parse_body(t: &str) -> IResult<&str, Box<dyn Renderer>> {
    let (rest, content) = take_while(|c| { c != '{'})(t)?;

    Ok((rest, Box::new(Body{ content: content.to_string()})))
}

fn parse_variable(t: &str) -> IResult<&str, Box<dyn Renderer>> {
    let (rest, var) = delimited(tag("{{"), take_until("}}"), tag("}}"))(t)?;
    
    Ok((rest, Box::new(Variable{name: var.trim().to_string()})))
}

fn parse_block(t: &str) -> IResult<&str, Box<dyn Renderer>> {
    let (rest, blockTag) = parse_block_tag(t)?;
    match blockTag {
        BlockTag::Loop(_) => {
            let (rest, (children, _)) = many_till(parse_content, tag("{% endfor %}"))(rest)?;
            Ok((rest, Box::new(Block {
                children,
                tag: blockTag
            })))
        },
        BlockTag::BlockName(_) => {
            let (rest, (children, _)) = many_till(parse_content, tag("{% endblock %}"))(rest)?;
            Ok((rest, Box::new(Block {
                children,
                tag: blockTag
            })))
        }
        BlockTag::Include(_) => {
            Ok((rest, Box::new(Block {children: Vec::default(), tag: blockTag})))
        }
        BlockTag::Undefined => {
            Ok((rest, Box::new(Block{children: Vec::default(), tag: blockTag})))
        }
    }
}

fn parse_block_tag(t: &str) -> IResult<&str, BlockTag> {
    let (rest, block) = delimited(tag("{%"), alt((parse_block_name, parse_block_loop, parse_block_include, parse_block_undefined)), tag("%}"))(t)?;

    Ok((rest, block))
}

fn parse_block_name(t: &str) -> IResult<&str, BlockTag> {
    let (rest, (_,_,_,name,_, _)) = tuple((multispace1, tag("block"), multispace1, take_till(|c| c == ' '), multispace1, take_until("%}")))(t)?;

    Ok((rest, BlockTag::BlockName(name.to_string())))
}

fn parse_block_include(t: &str) -> IResult<&str, BlockTag> {
    let (rest, (_,_include,_,_,name,_,_, _)) = tuple((multispace1, tag("include"), multispace1,one_of("'\""), take_till(|c| c == '\'' || c == '"'), one_of("'\""), multispace1, take_until("%}")))(t)?;

    Ok((rest, BlockTag::Include(name.to_string())))
}

fn parse_block_loop(t: &str) -> IResult<&str, BlockTag> {
    let (rest, (_,_for,_,name,_,_in,_,collection)) = tuple((multispace1, tag("for"), multispace1, take_till(|c| c == ' '), multispace1, tag("in"),multispace1, take_until("%}")))(t)?;
    let l = Loop{
        varname: name.trim().to_string(),
        collection_name: collection.trim().to_string()
    };

    Ok((rest, BlockTag::Loop(l)))
}

fn parse_block_undefined(t: &str) -> IResult<&str, BlockTag> {
    let (rest, _) = take_until("%}")(t)?;

    Ok((rest, BlockTag::Undefined))
}

trait Renderer {
    fn render(&self, buf: &mut String, env: &mut Environment);
}

impl Renderer for Variable {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        write!(buf,"{}", env.get(&self.name).unwrap_or_default());
    }
}

impl Renderer for Body {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        write!(buf, "{}", &self.content);
    }
}

impl Renderer for Block {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        env.push(StackFrame{variables: None, vals: HashMap::default()});
        match &self.tag {
            BlockTag::BlockName(_) => {
                self.children.render(buf, env)
            },
            BlockTag::Loop(lp) => {
                let collection = env.get_zval(&lp.collection_name).unwrap();
                let arr = collection.array().unwrap();
                for (_,_, val) in arr.iter() {
                    env.set(&lp.varname, val.string().unwrap());
                    self.children.render(buf, env)
                }
            }
            BlockTag::Include(template) =>  {
                let path = PathBuf::from(template);
                let mut file = File::open(&path).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content);
                match parse(&content) {
                    Ok((_, n)) => {
                        n.render(buf, env);
                    },
                    Err(e) => {write!(buf, "{}", e);},
                };
            }
            _=> ()}
    }
}

impl Renderer for NodeList {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        for x in self.iter() {
            x.render(buf, env)
        }
    }
}

impl Renderer for Module {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        self.children.render(buf, env)
    }
}

impl<'a> Environment<'a>  {
    fn push(&mut self, frame: StackFrame) {
        self.stack.push(frame)
    }

    fn get(&self, name: &str) -> Option<String> {
        self.stack.iter().rev().find_map(|frame| {
            frame.vals.get(name).map(String::to_owned)
                .or_else(|| get_rec(frame.variables.as_ref(), name).and_then(Zval::string))
        })
        .or_else(|| get_rec(Some(self.base), name).and_then(Zval::string))
    }


    fn get_zval(&self, name: &str) -> Option<Zval> {
        self.stack.iter().rev().find_map(|frame| {
            get_rec(frame.variables.as_ref(), name)
        }).or_else(|| get_rec(Some(self.base), name))
        .map(|z| z.shallow_clone())
    }


    fn set(&mut self, name: &str, value: String) {
        self.stack.last_mut().unwrap().vals.insert(name.to_string(), value);
    }
}

fn get_rec<'a>(map: Option<&'a Zval>, accessor: &'_ str) -> Option<&'a Zval> {
    if accessor.is_empty() {
        return map;
    }
    let map = map?;
    let (key, rest) = if accessor.contains('.') {
        accessor.split_once('.').unwrap()
    } else {
        (accessor, "")
    };


    if map.is_array() {
        let array = map.array().unwrap();
        return get_rec(array.get(key), rest);
    }

    if map.is_object() {
        let obj = map.object().unwrap();
        return obj.get_property(key).ok().and_then(|prop| {get_rec(Some(prop), rest)})
    }
    None
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
