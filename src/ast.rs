#[derive(Debug, Clone)]
pub struct Ast {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    VariableDeclaration {
        name: String,
        initializer: Option<Box<Expression>>,
        data_type: Option<Type>,
        is_mutable: bool,
        line: usize,
        column: usize,
    },
    FunctionDeclaration {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Vec<Box<Node>>,
        line: usize,
        column: usize,
    },
    StructDeclaration {
        name: String,
        fields: Vec<StructField>,
        line: usize,
        column: usize,
    },
    ClassDeclaration {
        name: String,
        methods: Vec<Box<Node>>,
        properties: Vec<StructField>,
        line: usize,
        column: usize,
    },
    ImportDeclaration {
        path: String,
        imported_items: Vec<String>,
        line: usize,
        column: usize,
    },
    ExpressionStatement {
        expression: Box<Expression>,
        line: usize,
        column: usize,
    },
    ReturnStatement {
        expression: Option<Box<Expression>>,
        line: usize,
        column: usize,
    },
    IfStatement {
        condition: Box<Expression>,
        then_branch: Vec<Box<Node>>,
        else_branch: Option<Vec<Box<Node>>>,
        line: usize,
        column: usize,
    },
    WhileStatement {
        condition: Box<Expression>,
        body: Vec<Box<Node>>,
        line: usize,
        column: usize,
    },
    ForStatement {
        initializer: Option<Box<Node>>,
        condition: Option<Box<Expression>>,
        increment: Option<Box<Expression>>,
        body: Vec<Box<Node>>,
        line: usize,
        column: usize,
    },
    ForInStatement {
        variable: String,
        iterable: Box<Expression>,
        body: Vec<Box<Node>>,
        line: usize,
        column: usize,
    },
    Block {
        statements: Vec<Box<Node>>,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal {
        value: LiteralValue,
        line: usize,
        column: usize,
    },
    Variable {
        name: String,
        line: usize,
        column: usize,
    },
    BinaryOperation {
        operator: String,
        left: Box<Expression>,
        right: Box<Expression>,
        line: usize,
        column: usize,
    },
    UnaryOperation {
        operator: String,
        operand: Box<Expression>,
        line: usize,
        column: usize,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
        line: usize,
        column: usize,
    },
    PropertyAccess {
        object: Box<Expression>,
        property: String,
        line: usize,
        column: usize,
    },
    ArrayAccess {
        array: Box<Expression>,
        index: Box<Expression>,
        line: usize,
        column: usize,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
        line: usize,
        column: usize,
    },
    ArrayLiteral {
        elements: Vec<Expression>,
        line: usize,
        column: usize,
    },
    ObjectLiteral {
        properties: Vec<ObjectProperty>,
        line: usize,
        column: usize,
    },
    Lambda {
        params: Vec<Parameter>,
        body: Vec<Box<Node>>,
        return_type: Option<Type>,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub typ: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub typ: Option<Type>,
    pub initializer: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub key: String,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum Type {
    Basic(String),
    Array(Box<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Optional(Box<Type>),
    Union(Vec<Type>),
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Type::Basic(name) => name.clone(),
            Type::Array(elem_type) => format!("{}[]", elem_type.to_string()),
            Type::Function {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                format!(
                    "fn({}) -> {}",
                    param_strs.join(", "),
                    return_type.to_string()
                )
            }
            Type::Optional(inner) => format!("{}?", inner.to_string()),
            Type::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                type_strs.join(" | ")
            }
        }
    }
}
