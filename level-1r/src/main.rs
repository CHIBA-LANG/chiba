use chiba_level1r::{compile_expr, Expr};

fn main() {
    let expr = Expr::call(Expr::var("f"), Expr::var("x"));
    let output = compile_expr(&expr);
    print!("{}", output.render_visual());
}

