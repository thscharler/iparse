use crate::debug::restrict;
use crate::error::{DebugWidth, Expect, ParserError, Suggest};
use crate::Code;
use std::fmt;
use std::fmt::Debug;

impl<'s, C: Code> Debug for ParserError<'s, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match f.width() {
            None | Some(0) => debug_parse_of_error_short(f, self),
            Some(1) => debug_parse_of_error_medium(f, self),
            Some(2) => debug_parse_of_error_long(f, self),
            _ => Ok(()),
        }
    }
}

impl<'s, C: Code> Debug for Suggest<'s, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let w = f.width().into();
        write!(f, "{}:\"{}\"", self.code, restrict(w, self.span))?;
        Ok(())
    }
}

impl<'s, C: Code> Debug for Expect<'s, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let w = f.width().into();
        write!(f, "{}:\"{}\"", self.code, restrict(w, self.span))?;
        Ok(())
    }
}

fn debug_parse_of_error_short<'s, C: Code>(
    f: &mut impl fmt::Write,
    err: &ParserError<'s, C>,
) -> fmt::Result {
    write!(
        f,
        "ParserError {} \"{}\"",
        err.code,
        restrict(DebugWidth::Short, err.span)
    )?;

    let nom = err.nom();
    if !nom.is_empty() {
        write!(f, " nom=")?;
        for n in &nom {
            write!(
                f,
                " {:?}:\"{}\"",
                n.kind,
                restrict(DebugWidth::Short, n.span)
            )?;
        }
    }

    let expect = err.expect();
    if !expect.is_empty() {
        write!(f, " expect=")?;
        debug_expect2_short(f, &expect, 1)?;
    }

    let suggest = err.suggest();
    if !suggest.is_empty() {
        write!(f, " suggest=")?;
        debug_suggest2_short(f, &suggest, 1)?;
    }

    Ok(())
}

fn debug_parse_of_error_medium<'s, C: Code>(
    f: &mut impl fmt::Write,
    err: &ParserError<'s, C>,
) -> fmt::Result {
    writeln!(
        f,
        "ParserError {} \"{}\"",
        err.code,
        restrict(DebugWidth::Medium, err.span)
    )?;

    let nom = err.nom();
    if !nom.is_empty() {
        writeln!(f, "nom=")?;
        for n in &nom {
            indent(f, 1)?;
            writeln!(
                f,
                "{:?}:\"{}\"",
                n.kind,
                restrict(DebugWidth::Medium, n.span)
            )?;
        }
    }

    let expect = err.expect();
    if !expect.is_empty() {
        let mut sorted = expect.clone();
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_offset = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_offset() != grp_offset {
                if !subgrp.is_empty() {
                    grp.push((grp_offset, subgrp));
                    subgrp = Vec::new();
                }
                grp_offset = exp.span.location_offset();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_offset, subgrp));
        }

        for (g_off, subgrp) in grp {
            let first = subgrp.first().unwrap();
            writeln!(
                f,
                "expect {}:\"{}\" ",
                g_off,
                restrict(DebugWidth::Medium, first.span)
            )?;
            debug_expect2_medium(f, &subgrp, 1)?;
        }
    }

    let suggest = err.suggest();
    if !suggest.is_empty() {
        let mut sorted = suggest.clone();
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_offset = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_offset() != grp_offset {
                if !subgrp.is_empty() {
                    grp.push((grp_offset, subgrp));
                    subgrp = Vec::new();
                }
                grp_offset = exp.span.location_offset();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_offset, subgrp));
        }

        for (g_off, subgrp) in grp {
            let first = subgrp.first().unwrap();
            writeln!(
                f,
                "suggest {}:\"{}\"",
                g_off,
                restrict(DebugWidth::Medium, first.span)
            )?;
            debug_suggest2_medium(f, &subgrp, 1)?;
        }
    }

    Ok(())
}

fn debug_parse_of_error_long<'s, C: Code>(
    f: &mut impl fmt::Write,
    err: &ParserError<'s, C>,
) -> fmt::Result {
    writeln!(
        f,
        "ParserError {} \"{}\"",
        err.code,
        restrict(DebugWidth::Long, err.span)
    )?;

    let nom = err.nom();
    if !nom.is_empty() {
        writeln!(f, "nom=")?;
        for n in &nom {
            indent(f, 1)?;
            writeln!(f, "{:?}:\"{}\"", n.kind, restrict(DebugWidth::Long, n.span))?;
        }
    }

    let expect = err.expect();
    if !expect.is_empty() {
        let mut sorted = expect.clone();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        writeln!(f, "expect=")?;
        debug_expect2_long(f, &sorted, 1)?;
    }

    let suggest = err.suggest();
    if !suggest.is_empty() {
        writeln!(f, "suggest=")?;
        debug_suggest2_long(f, &suggest, 1)?;
    }

    Ok(())
}

fn indent(f: &mut impl fmt::Write, ind: usize) -> fmt::Result {
    write!(f, "{}", " ".repeat(ind * 4))?;
    Ok(())
}

// expect2

fn debug_expect2_long<C: Code>(
    f: &mut impl fmt::Write,
    exp_vec: &Vec<&Expect<'_, C>>,
    ind: usize,
) -> fmt::Result {
    for exp in exp_vec {
        indent(f, ind)?;
        write!(
            f,
            "{}:{}:\"{}\"",
            exp.code,
            exp.span.location_offset(),
            restrict(DebugWidth::Long, exp.span)
        )?;
        if !exp.parents.is_empty() {
            write!(f, " <")?;
            for (i, p) in exp.parents.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", p)?;
            }
            write!(f, ">")?;
        }
        writeln!(f)?;
    }

    Ok(())
}

fn debug_expect2_medium<C: Code>(
    f: &mut impl fmt::Write,
    exp_vec: &Vec<&Expect<'_, C>>,
    ind: usize,
) -> fmt::Result {
    let mut prefix: Vec<Vec<C>> = Vec::new();

    for exp in exp_vec {
        indent(f, ind)?;
        write!(f, "{:20}", exp.code)?;

        let (suffix, sigil) = loop {
            if let Some(last) = prefix.last() {
                match exp.parents.strip_prefix(last.as_slice()) {
                    None => {
                        match prefix.pop() {
                            None => {
                                prefix.push(exp.parents.clone());
                                break (exp.parents.as_slice(), "<<");
                            }
                            Some(_) => {}
                        };
                    }
                    Some(suffix) => {
                        prefix.push(exp.parents.clone());
                        break (suffix, "++");
                    }
                }
            } else {
                prefix.push(exp.parents.clone());
                break (exp.parents.as_slice(), "<<");
            }
        };

        write!(f, " {} ", sigil)?;
        for (i, p) in suffix.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", p)?;
        }
        writeln!(f)?;
    }

    Ok(())
}

fn debug_expect2_short<C: Code>(
    f: &mut impl fmt::Write,
    exp_vec: &Vec<&Expect<'_, C>>,
    _ind: usize,
) -> fmt::Result {
    for exp in exp_vec {
        write!(
            f,
            "{}:\"{}\"",
            exp.code,
            restrict(DebugWidth::Short, exp.span)
        )?;
        if !exp.parents.is_empty() {
            write!(f, " <")?;
            for (i, p) in exp.parents.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", p)?;
            }
            write!(f, ">")?;
        }
    }

    Ok(())
}

// suggest2

fn debug_suggest2_long<C: Code>(
    f: &mut impl fmt::Write,
    sug_vec: &Vec<&Suggest<'_, C>>,
    ind: usize,
) -> fmt::Result {
    for sug in sug_vec {
        indent(f, ind)?;
        write!(
            f,
            "{}:{}:\"{}\"",
            sug.code,
            sug.span.location_offset(),
            restrict(DebugWidth::Long, sug.span)
        )?;
        if !sug.parents.is_empty() {
            write!(f, " <")?;
            for (i, p) in sug.parents.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", p)?;
            }
            write!(f, ">")?;
        }
        writeln!(f)?;
    }

    Ok(())
}

fn debug_suggest2_medium<C: Code>(
    f: &mut impl fmt::Write,
    sug_vec: &Vec<&Suggest<'_, C>>,
    ind: usize,
) -> fmt::Result {
    let mut prefix: Vec<Vec<C>> = Vec::new();

    for sug in sug_vec {
        indent(f, ind)?;
        write!(f, "{:20}", sug.code)?;

        let (suffix, sigil) = loop {
            if let Some(last) = prefix.last() {
                match sug.parents.strip_prefix(last.as_slice()) {
                    None => {
                        match prefix.pop() {
                            None => {
                                prefix.push(sug.parents.clone());
                                break (sug.parents.as_slice(), "<<");
                            }
                            Some(_) => {}
                        };
                    }
                    Some(suffix) => {
                        prefix.push(sug.parents.clone());
                        break (suffix, "++");
                    }
                }
            } else {
                prefix.push(sug.parents.clone());
                break (sug.parents.as_slice(), "<<");
            }
        };

        write!(f, " {} ", sigil)?;
        for (i, p) in suffix.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", p)?;
        }
        writeln!(f)?;
    }

    Ok(())
}

fn debug_suggest2_short<C: Code>(
    f: &mut impl fmt::Write,
    sug_vec: &Vec<&Suggest<'_, C>>,
    _ind: usize,
) -> fmt::Result {
    for sug in sug_vec {
        write!(
            f,
            "{}:\"{}\"",
            sug.code,
            restrict(DebugWidth::Short, sug.span)
        )?;
        if !sug.parents.is_empty() {
            write!(f, " <")?;
            for (i, p) in sug.parents.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", p)?;
            }
            write!(f, ">")?;
        }
    }

    Ok(())
}
