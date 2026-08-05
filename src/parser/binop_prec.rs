
fn binop_prec(op: &BinOp) -> u8 {
    match op {
        BinOp::DotDot | BinOp::DotDotEq => 1,
        BinOp::Or => 2,
        BinOp::And => 3,
        BinOp::Eq | BinOp::Neq => 4,
        BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => 5,
        BinOp::Add | BinOp::Sub => 6,
        BinOp::Mul | BinOp::Div => 7,
    }
}

