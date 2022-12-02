use crate::error::DebugWidth;
use crate::tracer::{
    CTracer, DebugTrack, EnterTrack, ErrTrack, ExitTrack, ExpectTrack, OkTrack, StepTrack,
    SuggestTrack, Track,
};
use crate::{Code, FilterFn};
use std::fmt;

fn indent(f: &mut impl fmt::Write, ind: usize) -> fmt::Result {
    write!(f, "{}", " ".repeat(ind * 2))?;
    Ok(())
}

pub(crate) fn debug_tracer<'s, C: Code>(
    o: &mut impl fmt::Write,
    w: DebugWidth,
    trace: &CTracer<'s, C>,
    filter: FilterFn<'_, C>,
) -> fmt::Result {
    let mut ind = 0;

    writeln!(o, "trace")?;

    for t in &*trace.track.borrow() {
        match t {
            Track::Enter(_) => {
                if filter(t) {
                    ind += 1;
                    indent(o, ind)?;
                    debug_track(o, w, t)?;
                    writeln!(o)?;
                }
            }
            Track::Step(_)
            | Track::Debug(_)
            | Track::Expect(_)
            | Track::Suggest(_)
            | Track::Ok(_)
            | Track::Err(_) => {
                if filter(t) {
                    indent(o, ind)?;
                    debug_track(o, w, t)?;
                    writeln!(o)?;
                }
            }
            Track::Exit(_) => {
                if filter(t) {
                    // indent(f, ind)?;
                    // debug_track(f, w, t)?;
                    // writeln!(f)?;
                    ind -= 1;
                }
            }
        }
    }

    if !trace.func.borrow().is_empty() {
        write!(o, "    func=")?;
        for func in &*trace.func.borrow() {
            write!(o, "{:?} ", func)?;
        }
        writeln!(o)?;
    }

    if !trace.expect.borrow().is_empty() {
        write!(o, "    expect=")?;
        for exp in &*trace.expect.borrow() {
            writeln!(o, "{}: {:?}", exp.func, exp.list)?;
        }
        writeln!(o)?;
    }

    if !trace.suggest.borrow().is_empty() {
        write!(o, "    suggest=")?;
        for sug in &*trace.suggest.borrow() {
            writeln!(o, "{}: {:?}", sug.func, sug.list)?;
        }
        writeln!(o)?;
    }

    Ok(())
}

fn debug_track<C: Code>(f: &mut impl fmt::Write, w: DebugWidth, v: &Track<'_, C>) -> fmt::Result {
    match v {
        Track::Enter(v) => debug_enter(f, w, v),
        Track::Step(v) => debug_step(f, w, v),
        Track::Debug(v) => debug_debug(f, w, v),
        Track::Expect(v) => debug_expect(f, w, v),
        Track::Suggest(v) => debug_suggest(f, w, v),
        Track::Ok(v) => debug_ok(f, w, v),
        Track::Err(v) => debug_err(f, w, v),
        Track::Exit(v) => debug_exit(f, w, v),
    }
}

fn debug_enter<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &EnterTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium => {
            write!(f, "{}: parse \"{}\"", v.func, v.span.escape_default())
        }
        DebugWidth::Long => write!(
            f,
            "{}: parse \"{}\" <<{:?}",
            v.func,
            v.span.escape_default(),
            v.parents
        ),
    }
}

fn debug_step<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &StepTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium => {
            write!(f, "{}: {} \"{}\"", v.func, v.step, v.span.escape_default())
        }
        DebugWidth::Long => {
            write!(
                f,
                "{}: {} \"{}\" <<{:?}",
                v.func,
                v.step,
                v.span.escape_default(),
                v.parents
            )
        }
    }
}

fn debug_debug<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &DebugTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium => write!(f, "{}: {}", v.func, v.dbg),
        DebugWidth::Long => write!(f, "{}: {} <<{:?}", v.func, v.dbg, v.parents),
    }
}

fn debug_expect<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &ExpectTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short => write!(f, "{}: {} expect {:?}", v.func, v.usage, v.list),
        DebugWidth::Medium => write!(f, "{}: {} expect {:?}", v.func, v.usage, v.list),
        DebugWidth::Long => write!(f, "{}: {} expect {:?}", v.func, v.usage, v.list),
    }
}

fn debug_suggest<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &SuggestTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short => write!(f, "{}: {} suggest {:?}", v.func, v.usage, v.list),
        DebugWidth::Medium => write!(f, "{}: {} suggest {:?}", v.func, v.usage, v.list),
        DebugWidth::Long => write!(f, "{}: {} suggest {:?}", v.func, v.usage, v.list),
    }
}
fn debug_ok<C: Code>(f: &mut impl fmt::Write, w: DebugWidth, v: &OkTrack<'_, C>) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium | DebugWidth::Long => {
            if !v.span.is_empty() {
                write!(
                    f,
                    "{}: -> [ {}, '{}' ]",
                    v.func,
                    v.span.escape_default(),
                    v.rest.escape_default()
                )?;
            } else {
                write!(f, "{}: -> no match", v.func)?;
            }
        }
    }
    Ok(())
}

fn debug_err<C: Code>(f: &mut impl fmt::Write, w: DebugWidth, v: &ErrTrack<'_, C>) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium => write!(f, "{}: {} ", v.func, v.err),
        DebugWidth::Long => write!(f, "{}: {} <<{:?}", v.func, v.err, v.parents),
    }
}

fn debug_exit<C: Code>(
    f: &mut impl fmt::Write,
    w: DebugWidth,
    v: &ExitTrack<'_, C>,
) -> fmt::Result {
    match w {
        DebugWidth::Short | DebugWidth::Medium | DebugWidth::Long => {
            write!(f, "return {}: ", v.func)
        }
    }
}
