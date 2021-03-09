use pest::{iterators::QueueableToken, ParseResult as PestResult, ParserState};

use super::Rule;

type PSR<'a> = Box<ParserState<'a, Rule>>;

/// This is a hack around the literal syntax to allow us to parse characters statefully.
pub(crate) fn literal_internal(state: PSR) -> PestResult<PSR> {
    use pest::Atomicity;

    // yoinked from the generated code
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn digit(state: PSR) -> PestResult<PSR> {
        state.match_range('\u{30}'..'\u{39}')
    }
    #[inline]
    #[allow(non_snake_case, unused_variables)]
    pub fn number(state: PSR) -> PestResult<PSR> {
        state.rule(Rule::number, |state| {
            state.sequence(|state| digit(state).and_then(|state| state.repeat(digit)))
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

    let state: PSR = state.match_string("{").and_then(number)?;
    let num_chars = {
        let queue = state.queue();
        let (start_idx, end_pos) = queue
            .iter()
            .rev()
            .find_map(|p| match p {
                QueueableToken::End {
                    start_token_index: start,
                    rule: Rule::number,
                    input_pos: pos,
                } => Some((*start, *pos)),
                _ => None,
            })
            .unwrap();
        let start_pos = match queue[start_idx] {
            QueueableToken::Start { input_pos: pos, .. } => pos,
            _ => unreachable!(),
        };

        let inp = state.position().get_str();
        let seg = &inp[start_pos..end_pos];
        match seg.parse::<usize>() {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "failed to parse int from {}..{} {:?}: {}",
                    start_pos, end_pos, seg, e
                );
                return Err(state);
            }
        }
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
}

pub(crate) fn noop(state: PSR) -> PestResult<PSR> {
    // TODO: probably should be unreachable?
    Ok(state)
}
