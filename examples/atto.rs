use zub::{ir::*, vm::*};

fn parse_expr(
    builder: &mut IrBuilder,
    slice: &mut &[&str],
    get_binding: &impl Fn(&str) -> Option<usize>,
) -> Option<Node<Expr>> {
    match *slice {
        [] => None,
        [ident, ..] => {
            *slice = &slice[1..];
            if *ident == "if" {
                let cond = parse_expr(builder, slice, get_binding)?;
                let a = parse_expr(builder, slice, get_binding)?;
                let b = parse_expr(builder, slice, get_binding)?;
                Some(builder.ternary(cond, a, Some(b)))
            } else if let Some(op) = match *ident {
                "+" => Some(BinaryOp::Add),
                "-" => Some(BinaryOp::Sub),
                "*" => Some(BinaryOp::Mul),
                "/" => Some(BinaryOp::Div),
                //"%" => Some(BinaryOp::Rem),
                "=" => Some(BinaryOp::Equal),
                ">" => Some(BinaryOp::Gt),
                "<" => Some(BinaryOp::Lt),
                ">=" => Some(BinaryOp::GtEqual),
                "<=" => Some(BinaryOp::LtEqual),
                "&" => Some(BinaryOp::And),
                "|" => Some(BinaryOp::Or),
                _ => None,
            } {
                let a = parse_expr(builder, slice, get_binding)?;
                let b = parse_expr(builder, slice, get_binding)?;
                Some(builder.binary(a, op, b))
            } else if let Ok(n) = ident.parse() {
                Some(builder.number(n))
            } else if let Some(val) = match *ident {
                "true" => Some(builder.bool(true)),
                "false" => Some(builder.bool(false)),
                //"null" => Some(builder.nil()),
                _ => None,
            } {
                Some(val)
            } else if let Some(args) = get_binding(ident) {
                let args = (0..args).map(|_| parse_expr(builder, slice, get_binding)).collect::<Option<_>>()?;
                Some(builder.call(
                    builder.var(Binding::local(ident, 1, 1)),
                    args,
                    None,
                ))
            } else {
                None
            }
        },
    }
}

fn parse_fn<'a>(
    builder: &mut IrBuilder,
    slice: &mut &'a [&'a str],
    get_binding: &impl Fn(&str) -> Option<usize>,
) -> Option<(&'a str, usize)> {
    match *slice {
        [] => None,
        ["fn", name, ..] => {
            let params = slice[2..]
                .into_iter()
                .take_while(|token| **token != "is")
                .copied()
                .collect::<Vec<_>>();

            *slice = &slice[3 + params.len()..];

            let func = builder.function(
                Binding::local(*name, 0, 0),
                &params,
                |builder| {
                    let body = parse_expr(builder, slice, &|ident| if ident == *name {
                        Some(params.len())
                    } else if params.contains(&&ident) {
                        Some(0)
                    } else {
                        get_binding(ident)
                    });

                    builder.ret(Some(body.unwrap()));
                },
            );

            builder.emit(func);

            Some((*name, params.len()))
        },
        _ => panic!("Not a function: {:?}", slice),
    }
}

const CODE: &'static str = r#"
fn sum x is
    if = x 0
        1
    + sum - x 1 sum - x 1

fn main is
    sum 12
"#;

fn main() {
    let tokens = CODE.split_whitespace().collect::<Vec<_>>();

    let mut builder = IrBuilder::new();
    let mut fns = Vec::<(&str, usize)>::new();
    let mut token_slice = &tokens[..];
    while let Some((name, args)) = parse_fn(&mut builder, &mut token_slice, &|ident| {
        fns.iter().rev().find(|f| f.0 == ident).map(|f| f.1)
    }) {
        fns.push((name, args));
    }

    let main_var = builder.var(Binding::global("main"));
    let main_call = builder.call(main_var, vec![], None);
    builder.bind(Binding::global("entry"), main_call);

    let build = builder.build();

    // println!("{:#?}", build);
    // println!();
    // println!();

    let mut vm = VM::new();
    vm.exec(&build);
    println!("{:?}", vm.globals["entry"]);
}
