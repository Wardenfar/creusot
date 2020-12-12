use std::fmt;

use super::*;

/// Original code from https://github.com/digama0/mm0/ (CC-0)
/// The side information required to print an object in the environment.

#[derive(Copy, Clone, Debug)]
pub struct FormatEnv<'a> {
    /// Currently open scopes.
    pub scope: &'a [String],
    /// Indentation to prefix lines with
    pub indent: usize,
}

/// A trait for displaying data given access to the environment.
pub trait EnvDisplay {
    /// Print formatted output to the given formatter. The signature is exactly the same
    /// as [`Display::fmt`] except it has an extra argument for the environment.
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// The result of [`FormatEnv::to`], a struct that implements [`Display`] if the
/// argument implements [`EnvDisplay`].
pub struct Print<'a, D: ?Sized> {
    fe: FormatEnv<'a>,
    e: &'a D,
}

impl<'a> FormatEnv<'a> {
    /// Given a [`FormatEnv`], convert an `impl EnvDisplay` into an `impl Display`.
    /// This can be used in macros like `println!("{}", fe.to(e))` to print objects.
    pub fn to<D: ?Sized>(self, e: &'a D) -> Print<'a, D> {
        Print { fe: self, e }
    }

    pub fn indent<F>(mut self, i : usize, mut f : F) -> std::fmt::Result
    where F: FnMut(Self) -> std::fmt::Result
    {
      self.indent += i;
      f(self)
    }

    // Print the correct indentation for this line
    pub fn indent_line(self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f,"{:indent$}", "", indent = self.indent)
    }
}

impl<'a, D: EnvDisplay + ?Sized> fmt::Display for Print<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.e.fmt(self.fe, f)
    }
}

// FIXME: Doesn't take into account associativity when deciding when to put parens
macro_rules! parens {
    ($fe:ident, $e:ident, $i:expr) => {
        if $i.precedence() < $e.precedence() {
            format!("({})", $fe.to($i))
        } else {
            format!("{}", $fe.to($i))
        }
    };
}

impl EnvDisplay for Function {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fe.indent_line(f)?;
        write!(f, "let cfg {} ", fe.to(&self.name))?;

        if self.args.is_empty() {
            write!(f, "()")?;
        }

        for (nm, ty) in &self.args {
            write!(f, "(o_{} : {})", nm, fe.to(ty))?;
        }

        writeln!(f, " : {}", fe.to(&self.retty))?;

        fe.indent(2, |fe| {
            for req in &self.preconds {
                fe.indent_line(f)?;
                writeln!(f, "requires {{ {} }}", req)?;
            }

            for req in &self.postconds {
                fe.indent_line(f)?;
                writeln!(f, "ensures {{ {} }}", req)?;
            }
            fe.indent_line(f)?;
            writeln!(f, "=")?;

            Ok(())
        })?;

        // Forward declare all arguments
        fe.indent_line(f)?;
        writeln!(f, "var _0 : {};", fe.to(&self.retty))?;

        for (var, ty) in self.args.iter() {
            fe.indent_line(f)?;
            writeln!(f, "var {} : {};", var, fe.to(ty))?;
        }

        // Forward declare all variables
        for (var, ty) in self.vars.iter() {
            fe.indent_line(f)?;
            writeln!(f, "var {} : {};", var, fe.to(ty))?;
        }

        fe.indent_line(f)?;
        writeln!(f, "{{")?;
        fe.indent(2, |fe| {
          for (arg, _) in self.args.iter() {
              fe.indent_line(f)?;
              writeln!(f, "{} <- o_{};", arg, arg)?;
          }

          fe.indent_line(f)?;
          writeln!(f, "goto BB0;")
        })?;

        fe.indent_line(f)?;
        writeln!(f, "}}")?;

        for block in &self.blocks {
            write!(f, "{}", fe.to(block))?;
        }

        Ok(())
    }
}

impl EnvDisplay for Type {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Type::*;

        if self.complex() {
            write!(f, "(")?;
        }
        match self {
            Bool => {
                write!(f, "bool")?;
            }
            Char => {
                write!(f, "char")?;
            }
            Int(size) => {
                use rustc_ast::ast::IntTy::*;
                match size {
                    I8      => write!(f, "int8"),
                    I16     => write!(f, "int16"),
                    I32     => write!(f, "int32"),
                    I64     => write!(f, "int64"),
                    I128    => write!(f, "int128"),
                    Isize   => write!(f, "isize"),
                }?
            }
            Uint(size) => {
                use rustc_ast::ast::UintTy::*;
                match size {
                    U8      => write!(f, "uint8"),
                    U16     => write!(f, "uint16"),
                    U32     => write!(f, "uint32"),
                    U64     => write!(f, "uint64"),
                    U128    => write!(f, "uint128"),
                    Usize   => write!(f, "usize"),
                }?
            }
            Float(size) => {
                use rustc_ast::ast::FloatTy::*;
                match size {
                    F32 => write!(f, "single"),
                    F64 => write!(f, "double"),
                }?
            }
            MutableBorrow(t) => {
                write!(f, "borrowed {}", fe.to(&**t))?;
            }
            TVar(v) => {
                write!(f, "{}", v)?;
            }
            TConstructor(ty) => {
                write!(f, "{}", fe.to(ty))?;
            }
            TApp(tyf, args) => {
                write!(f, "{} {}", fe.to(&**tyf), args.iter().format_with(" ", |elt, f| {
                  f(&format_args!("{}", fe.to(elt)))
                }))?;
            }
            Tuple(tys) => {
                write!(f, "({})", tys.iter().format_with(" ", |elt, f| {
                  f(&format_args!("{}", fe.to(elt)))
                }))?;
            }
        }
        if self.complex() {
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl EnvDisplay for Exp {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exp::Current(box e) => {
                write!(f, " * {}", fe.to(e))?;
            }
            Exp::Final(box e) => {
                write!(f, " ^ {}", fe.to(e))?;
            }
            Exp::Let { pattern, box arg, box body } => {
                write!(f, "let {} = {} in {}", pattern, parens!(fe, self, arg), parens!(fe, self, body))?;
            }
            Exp::Var(v) => {
                write!(f, "{}", v)?;
            }
            // Exp::QVar(v) => {
            //     write!(f, "{}", v)?;
            // }
            Exp::RecUp { box record, label, box val } => {
                write!(f, "{{ {} with {} = {} }}", parens!(fe, self, record), label, parens!(fe, self, val))?;
            }
            Exp::Tuple(vs) => {
                write!(f, "({})", vs.iter().format_with(", ", |elt, f| { f(&format_args!("{}", fe.to(elt)))}))?;
            }
            Exp::Constructor { ctor, args } => {
                if args.is_empty() {
                    EnvDisplay::fmt(ctor, fe, f)?;
                } else {
                    write!(f, "{}({})", ctor, args.iter().format_with(", ", |elt, f| { f(&format_args!("{}", fe.to(elt)))}))?;
                }
            }
            Exp::BorrowMut(box exp) => {
                write!(f, "borrow_mut {}", parens!(fe, self, exp))?;
            }
            Exp::Const(c) => {
                write!(f, "{}", c)?;
            }
            Exp::BinaryOp(FullBinOp::Other(BinOp::Div), box l, box r) => {
                write!(f, "div {} {}", parens!(fe, self, l), parens!(fe, self, r))?;
            }
            Exp::BinaryOp(op, box l, box r) => {
                write!(f, "{} {} {}", parens!(fe, self, l), bin_op_to_string(op), parens!(fe, self, r))?;
            }
            Exp::Call(fun, args) => {
                write!(f, "{} {}", fun, args.iter().map(|a| parens!(fe, self, a)).format(" "))?;
            }
            Exp::Verbatim(verb) => {
                write!(f, "{}", verb)?;
            }
            Exp::Forall(binders, box exp) => {
                write!(f, "forall ")?;

                for (l, ty) in binders {
                    write!(f, "({} : {}) ", l, fe.to(ty))?;
                }

                write!(f, ". {}", fe.to(exp))?;
            }
            Exp::Exists(binders, box exp) => {
                write!(f, "exists ")?;

                for (l, ty) in binders {
                    write!(f, "({} : {}) ", l, fe.to(ty))?;
                }

                write!(f, ". {}", fe.to(exp))?;
            }
            Exp::Impl(hyp, exp) => {
                write!(f, "{} -> {}", parens!(fe, self, &**hyp), parens!(fe, self, &**exp))?;
            }
        }
        Ok(())
    }
}

impl EnvDisplay for Statement {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fe.indent_line(f)?;
        match self {
            Statement::Assign { lhs, rhs } => {
                write!(f, "{} <- {}", lhs, fe.to(rhs))?;
            }
            Statement::Freeze(loc) => {
                write!(f, "assume {{ ^ {} = * {} }}", loc, loc)?;
            }
            Statement::Invariant(nm, e) => {
                write!(f, "invariant {} {{ {} }}", nm, fe.to(e))?;
            }
        }
        Ok(())
    }
}

impl EnvDisplay for Terminator {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Terminator::*;
        fe.indent_line(f)?;

        match self {
            Goto(tgt) => {
                writeln!(f, "goto {}", tgt)?;
            }
            Absurd => {
                writeln!(f, "absurd")?;
            }
            Return => {
                writeln!(f, "_0")?;
            }
            Switch(discr, brs) => {
                writeln!(f, "switch ({})", fe.to(discr))?;
                fe.indent(2, |fe| {
                  for (pat, tgt) in brs {
                      fe.indent_line(f)?;
                      writeln!(f, "| {} -> goto {}", pat, tgt)?;
                  }
                  fe.indent_line(f)?;
                  writeln!(f, "end")
                })?;
            }
        }
        Ok(())
    }
}


impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::Wildcard => {
                write!(f, "_")?;
            }
            Pattern::VarP(v) => {
                write!(f, "{}", v)?;
            }
            Pattern::TupleP(vs) => {
                write!(f, "({})", vs.iter().format(", "))?;
            }
            Pattern::ConsP(c, pats) => {
                if pats.is_empty() {
                    write!(f, "{}", c)?;
                } else {
                    write!(f, "{}({})", c, pats.iter().format(", "))?;
                }
            }
            Pattern::LitP(lit) => {
                write!(f, "{}", lit)?;
            }
        }
        Ok(())
    }
}

use itertools::*;

impl Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BB{}", self.0)
    }
}

impl EnvDisplay for Block {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fe.indent_line(f)?;
        writeln!(f, "{} {{", self.label)?;

        fe.indent(2, |fe| {
          for stmt in &self.statements {
              writeln!(f, "{};", fe.to(stmt))?;
          }

          self.terminator.fmt(fe, f)
        })?;

        fe.indent_line(f)?;
        writeln!(f, "}}")?;

        Ok(())
    }
}

fn bin_op_to_string(op: &FullBinOp) -> &str {
    use FullBinOp::*;
    use rustc_middle::mir::BinOp::*;
    match op {
        And => "&&",
        Or => "||",
        Other(Add) => "+",
        Other(Sub) => "-",
        Other(Mul) => "*",
        Other(Eq )=> "=",
        Other(Ne )=> "<>",
        Other(Gt )=> ">",
        Other(Ge )=> ">=",
        Other(Lt )=> "<",
        Other(Le )=> "<=",
        _ => unreachable!("unexpected bin-op"),
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl EnvDisplay for TyDecl {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fe.indent_line(f)?;
        writeln!(f, "type {} {} =", fe.to(&self.ty_name), self.ty_params.iter().format(" "))?;

        fe.indent(2, |fe| {
          for (cons, args) in self.ty_constructors.iter() {
              fe.indent_line(f)?;
              if args.is_empty() {
                  writeln!(f, "  | {}", cons)?;
              } else {
                  writeln!(f, "  | {}({})", cons, args.iter().format_with(", ", |elt, f| { f(&format_args!("{}", fe.to(elt)))}))?;
              }
          }
          Ok(())
        })?;

        Ok(())
    }
}

impl EnvDisplay for QName {
    fn fmt(&self, fe: FormatEnv, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Strip the shared prefix between currently open scope and the identifier we are printing
        let module_path = format!("{}", fe
            .scope
            .iter()
            .zip(self.module.iter())
            .skip_while(|(p, m)| p == m)
            .map(|t| t.1)
            .format("."));

        let ident = self.name.iter().format("_");

        if module_path == "" {
          write!(f, "{}", ident)
        } else {
          write!(f, "{}.{}", module_path, ident)
        }
    }
}