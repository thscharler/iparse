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
