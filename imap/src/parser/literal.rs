use pest::{ParseResult as PestResult, ParserState};

use super::old::Rule;

type PSR<'a> = Box<ParserState<'a, Rule>>;

pub(crate) fn literal_internal(state: PSR) -> PestResult<PSR> {
    use pest::Atomicity;

    // yoinked from the generated code
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn digit(state: PSR) -> PestResult<PSR> {
        state.rule(Rule::digit, |state| {
            state.atomic(Atomicity::Atomic, |state| {
                state.match_range('\u{30}'..'\u{39}')
            })
        })
    }
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn number(state: PSR) -> PestResult<PSR> {
        state.rule(Rule::number, |state| {
            state.atomic(Atomicity::Atomic, |state| {
                state.sequence(|state| {
                    digit(state).and_then(|state| state.repeat(|state| digit(state)))
                })
            })
        })
    }
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn char8(state: PSR) -> PestResult<PSR> {
        state.rule(Rule::char8, |state| {
            state.atomic(Atomicity::Atomic, |state| {
                state.match_range('\u{1}'..'\u{ff}')
            })
        })
    }
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn crlf(state: PSR) -> PestResult<PSR> {
        state.sequence(|state| state.match_string("\r")?.match_string("\n"))
    }

    let state = state.match_string("{").and_then(number)?;
    let num_chars = {
        let mut queue = state.queue().iter().rev();
        println!("QUEUE: {:?}", queue);
        let end = queue.next().unwrap();
        let start = queue.next().unwrap();
        let inp = state.position().get_str();
        let seg = &inp[start.input_pos()..end.input_pos()];
        seg.parse::<usize>().unwrap()
    };

    state
        .match_string("}")
        .and_then(crlf)?
        .rule(Rule::literal_str, |state| {
            state.atomic(Atomicity::Atomic, |state| {
                let mut state = Ok(state);
                for _ in 0..num_chars {
                    state = state.and_then(char8);
                }

                state
            })
        })
    // todo!("hit internal state: {:?}", state,);
}

pub(crate) fn noop(state: PSR) -> PestResult<PSR> {
    // TODO: probably should be unreachable?
    Ok(state)
}
