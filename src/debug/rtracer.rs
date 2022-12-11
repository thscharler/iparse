use crate::error::DebugWidth;
use crate::rtracer::RTracer;
use crate::Code;
use std::fmt;

pub(crate) fn debug_rtracer<'s, C: Code>(
    o: &mut impl fmt::Write,
    _w: DebugWidth,
    trace: &RTracer<'s, C>,
) -> fmt::Result {
    writeln!(o, "trace")?;

    if !trace.func.is_empty() {
        write!(o, "    func=")?;
        for func in &*trace.func {
            write!(o, "{:?} ", func)?;
        }
        writeln!(o)?;
    }

    if !trace.expect.is_empty() {
        write!(o, "    expect=")?;
        for exp in &*trace.expect {
            writeln!(o, "{}: {:?}", exp.func, exp.list)?;
        }
        writeln!(o)?;
    }

    if !trace.suggest.is_empty() {
        write!(o, "    suggest=")?;
        for sug in &*trace.suggest {
            writeln!(o, "{}: {:?}", sug.func, sug.list)?;
        }
        writeln!(o)?;
    }

    Ok(())
}
