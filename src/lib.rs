use std::{collections::{HashMap, VecDeque}, fs::{self, File}, path::{self, PathBuf, Path}, io::Read, fmt::Write, any::Any};

use ext_php_rs::{prelude::*, types::{ZendObject, Zval}, boxed::ZBox};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_until1, take_till, take_while},
    multi::{many0, many1, many_till},
    sequence::{delimited, tuple},
    IResult, character::{complete::{multispace1, one_of}, is_space},
};


type NodeList = Vec<Content>;

struct Module {
    children: NodeList
}

struct Extends {
    blocks: NodeList
}

#[derive(Debug)]
enum BlockTag {
    BlockName(String),
    Loop(Loop),
    Include(String),
    Extends(String),
    Undefined
}

#[derive(Debug)]
struct Loop {
    varname: String,
    collection_name: String
}

enum Content {
    Text(String),
    Var(String),
    Block(Block)
}

struct Block {
    tag: BlockTag,
    children: Vec<Content>
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
    match file_to_ast(&path) {
        Ok(n) => {
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

fn file_to_ast(path: &Path) -> Result<Module, Box<dyn std::error::Error> > {
    let mut f = File::open(path)?;
    let mut buf = String::default();
    f.read_to_string(&mut buf)?;
    parse(&buf).map(|(_,module)| module).map_err(|e| e.to_string().into())
}


fn parse(t: &str) -> IResult<&str, Module> {
    let mut n = Module {
        children: Vec::default(),
    };
    let (rest, (children, _)) = many_till(parse_content, nom::combinator::eof)(t)?;
    n.children = children;
    Ok((rest, n))
}

fn parse_content(t: &str) -> IResult<&str, Content> {

    let (rest, content) = alt((parse_variable, parse_block, parse_body))(t)?;
    Ok((rest, content))
}

fn parse_body(t: &str) -> IResult<&str, Content> {
    let (rest, content) = take_while(|c| { c != '{'})(t)?;

    Ok((rest, Content::Text(content.to_string())))
}

fn parse_variable(t: &str) -> IResult<&str, Content> {
    let (rest, var) = delimited(tag("{{"), take_until("}}"), tag("}}"))(t)?;
    
    Ok((rest, Content::Var(var.trim().to_string())))
}

fn parse_block(t: &str) -> IResult<&str, Content> {
    let (rest, blockTag) = parse_block_tag(t)?;
    match blockTag {
        BlockTag::Loop(_) => {
            let (rest, (children, _)) = many_till(parse_content, tag("{% endfor %}"))(rest)?;
            Ok((rest, Content::Block(Block {
                children,
                tag: blockTag,
            })))
        },
        BlockTag::BlockName(_) => {
            let (rest, (children, _)) = many_till(parse_content, tag("{% endblock %}"))(rest)?;
            Ok((rest, Content::Block(Block {
                children,
                tag: blockTag,
            })))
        }
        BlockTag::Extends(_) => {
            Ok((rest, Content::Block(Block {children: Vec::default(), tag: blockTag})))
        }
        BlockTag::Include(_) => {
            Ok((rest, Content::Block(Block {children: Vec::default(), tag: blockTag})))
        }
        BlockTag::Undefined => {
            Ok((rest, Content::Block(Block{children: Vec::default(), tag: blockTag})))
        }
    }
}

fn parse_block_tag(t: &str) -> IResult<&str, BlockTag> {
    let (rest, block) = delimited(tag("{%"), alt((parse_block_name, parse_block_loop, parse_block_include, parse_block_extends, parse_block_undefined)), tag("%}"))(t)?;

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

fn parse_block_extends(t: &str) -> IResult<&str, BlockTag> {
    let (rest, (_,_extends,_,_,name,_,_, _)) = tuple((multispace1, tag("extends"), multispace1,one_of("'\""), take_till(|c| c == '\'' || c == '"'), one_of("'\""), multispace1, take_until("%}")))(t)?;

    Ok((rest, BlockTag::Extends(name.to_string())))
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

impl Content {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        match self {
            Content::Text(txt) => write!(buf, "{}", txt),
            Content::Var(name) => write!(buf,"{}", env.get(name).unwrap_or_default()),
            Content::Block(block) => { block.render(buf, env); Ok(())}
        }.unwrap();
    }
}

impl Block {
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
                match file_to_ast(&path) {
                    Ok(n) => {
                        n.render(buf, env);
                    },
                    Err(e) => {write!(buf, "{}", e);},
                };
            }
            _=> ()}
    }

    fn block_name(&self) -> Option<String> {
        match &self.tag {
            BlockTag::BlockName(name) => Some(name.to_owned()),
            _ => None
        }
    }
}

trait Render {
    fn render(&self,buf: &mut String, env: &mut Environment);
}

impl Render for NodeList {
    fn render(&self, buf: &mut String, env: &mut Environment) {
        for x in self.iter() {
            x.render(buf, env)
        }
    }
}

impl Module {
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
