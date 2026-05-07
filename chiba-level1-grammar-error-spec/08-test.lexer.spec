Err(
    consumed=21,
    reason="unexpected token or incomplete construct",
    final_reason="final failure at next unconsumed token",
    prev_token=[5:1 len=1] Token,
    next_token=[0:0 len=0] Eof,
    partial_ast=SourceFile(
    Attr_End,
    Namespace(
      Path_Cons(
        "parser",
        Path_Cons(
          "error",
          Path_Cons(
            "trailing",
            Path_Cons(
              "operator",
              Path_End
            )
          )
        )
      )
    ),
    Item_Cons(
      Item_Def(
        DefItem2(
          "trailing_operator",
          DefFun(
            Generic_End,
            Param_End,
            Type_Path(
              "i64",
              Type_Tail_End
            ),
            Expr_Block(
              Null,
              TailExpr_None
            )
          )
        )
      ),
      Item_End
    )
  )
)
0
