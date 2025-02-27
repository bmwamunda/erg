use erg_common::traits::{DequeStream, Locational, NoTypeDisplay};
use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;
use erg_compiler::erg_parser::token::{Token, TokenKind};
use erg_compiler::hir::Expr;
use erg_compiler::ty::{HasType, ParamTy};

use lsp_types::{
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureHelpContext,
    SignatureHelpParams, SignatureHelpTriggerKind, SignatureInformation,
};

use crate::server::{send_log, ELSResult, Server};
use crate::util::NormalizedUrl;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Paren,
    Comma,
    VBar, // e.g. id|T := Int|
}

impl From<String> for Trigger {
    fn from(s: String) -> Self {
        match s.as_str() {
            "(" => Trigger::Paren,
            "," => Trigger::Comma,
            "|" => Trigger::VBar,
            _ => unreachable!(),
        }
    }
}

fn get_end(start: usize, pt: &ParamTy) -> usize {
    start + pt.name().map(|n| n.len() + 2).unwrap_or(0) + pt.typ().to_string().len()
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_signature_help(
        &mut self,
        params: SignatureHelpParams,
    ) -> ELSResult<Option<SignatureHelp>> {
        send_log(format!("signature help requested: {params:?}"))?;
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        if params.context.as_ref().map(|ctx| &ctx.trigger_kind)
            == Some(&SignatureHelpTriggerKind::CONTENT_CHANGE)
        {
            let Some(ctx) = params.context.as_ref() else {
                return Ok(None);
            };
            let help = self.resend_help(&uri, pos, ctx);
            return Ok(help);
        }
        let trigger = params
            .context
            .and_then(|c| c.trigger_character)
            .map(Trigger::from);
        let result = match trigger {
            Some(Trigger::Paren) => self.get_first_help(&uri, pos),
            Some(Trigger::Comma) => self.get_continuous_help(&uri, pos),
            Some(Trigger::VBar) | None => None,
        };
        Ok(result)
    }

    pub(crate) fn get_min_expr(
        &self,
        uri: &NormalizedUrl,
        pos: Position,
        offset: isize,
    ) -> Option<(Token, Expr)> {
        let token = self.file_cache.get_token_relatively(uri, pos, offset)?;
        crate::_log!("token: {token}");
        if let Some(visitor) = self.get_visitor(uri) {
            #[allow(clippy::single_match)]
            match visitor.get_min_expr(&token) {
                Some(expr) => {
                    return Some((token, expr.clone()));
                }
                _ => {}
            }
        }
        None
    }

    pub(crate) fn nth(
        &self,
        uri: &NormalizedUrl,
        args_loc: erg_common::error::Location,
        token: &Token,
    ) -> usize {
        let tks = self.file_cache.get_token_stream(uri).unwrap_or_default();
        // we should use the latest commas
        let commas = tks
            .iter()
            .skip_while(|&tk| tk.loc() < args_loc)
            .filter(|tk| tk.is(TokenKind::Comma) && args_loc.ln_end() >= tk.ln_begin())
            .collect::<Vec<_>>();
        let argc = commas.len();
        commas
            .iter()
            .position(|c| c.col_end() >= token.col_end())
            .unwrap_or(argc) // `commas.col_end() < token.col_end()` means the token is the last argument
    }

    fn resend_help(
        &mut self,
        uri: &NormalizedUrl,
        pos: Position,
        ctx: &SignatureHelpContext,
    ) -> Option<SignatureHelp> {
        if let Some(token) = self.file_cache.get_token(uri, pos) {
            crate::_log!("token: {token}");
            if let Some(Expr::Call(call)) = &self.current_sig {
                if call.ln_begin() > token.ln_begin() || call.ln_end() < token.ln_end() {
                    self.current_sig = None;
                    return None;
                }
                let nth = self.nth(uri, call.args.loc(), &token) as u32;
                return self.make_sig_help(call.obj.as_ref(), nth);
            }
        } else {
            crate::_log!("failed to get the token");
        }
        ctx.active_signature_help.clone()
    }

    fn get_first_help(&mut self, uri: &NormalizedUrl, pos: Position) -> Option<SignatureHelp> {
        if let Some((_token, Expr::Accessor(acc))) = self.get_min_expr(uri, pos, -2) {
            return self.make_sig_help(&acc, 0);
        } else {
            crate::_log!("lex error occurred");
        }
        None
    }

    fn get_continuous_help(&mut self, uri: &NormalizedUrl, pos: Position) -> Option<SignatureHelp> {
        if let Some((comma, Expr::Call(call))) = self.get_min_expr(uri, pos, -1) {
            let nth = self.nth(uri, call.args.loc(), &comma) as u32 + 1;
            let help = self.make_sig_help(call.obj.as_ref(), nth);
            self.current_sig = Some(Expr::Call(call));
            return help;
        } else {
            crate::_log!("failed to get continuous help");
        }
        None
    }

    fn make_sig_help<S: HasType + NoTypeDisplay>(
        &self,
        sig: &S,
        nth: u32,
    ) -> Option<SignatureHelp> {
        let sig_t = sig.ref_t();
        let mut parameters = vec![];
        let sig = sig.to_string_notype();
        let label = format!("{sig}: {sig_t}");
        let mut end = sig.len() + 1; // +1: (
        for nd_param in sig_t.non_default_params()? {
            let start = end + 2;
            end = get_end(start, nd_param);
            let param_info = ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None, //Some(Documentation::String(nd_param.typ().to_string())),
            };
            parameters.push(param_info);
        }
        if let Some(var_params) = sig_t.var_params() {
            let start = end + 2;
            end = get_end(start, var_params);
            let param_info = ParameterInformation {
                label: ParameterLabel::LabelOffsets([start as u32, end as u32]),
                documentation: None, //Some(Documentation::String(var_params.typ().to_string())),
            };
            parameters.push(param_info);
        }
        let nth = (parameters.len() as u32 - 1).min(nth);
        let info = SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(nth),
        };
        Some(SignatureHelp {
            signatures: vec![info],
            active_parameter: None,
            active_signature: None,
        })
    }
}
