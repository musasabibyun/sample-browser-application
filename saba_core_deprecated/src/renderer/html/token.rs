use crate::renderer::html::attribute::Attribute;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    // 開始タグ
    StartTag {
        tag: String,
        self_closing: bool,
        attributes: Vec<Attribute>,
    },
    // 終了タグ
    EndTag {
        tag: String,
    },
    // 文字
    Char(char),
    // ファイルの終了（End Of File）
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// https://html.spec.whatwg.org/multipage/parsing.html#data-state
    Data,
    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
    TagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
    EndTagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
    TagName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
    BeforeAttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
    AttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
    AfterAttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
    BeforeAttributeValue,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
    AttributeValueDoubleQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
    AttributeValueSingleQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
    AttributeValueUnquoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
    AfterAttributeValueQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
    SelfClosingStartTag,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-state
    ScriptData,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-less-than-sign-state
    ScriptDataLessThanSign,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-end-tag-open-state
    ScriptDataEndTagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-end-tag-name-state
    ScriptDataEndTagName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#temporary-buffer
    TemporaryBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlTokenizer {
    state: State,
    pos: usize,
    reconsume: bool,
    latest_token: Option<HtmlToken>,
    input: Vec<char>,
    buf: String,
}

impl HtmlTokenizer {
    pub fn new(html: String) -> Self {
        Self {
            state: State::Data,
            pos: 0,
            reconsume: false,
            latest_token: None,
            input: html.chars().collect(),
            buf: String::new(),
        }
    }
    fn consume_next_input(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }
    fn create_tag(&mut self, start_tag_token: bool) {
        if start_tag_token {
            self.latest_token = Some(HtmlToken::StartTag {
                tag: String::new(),
                self_closing: false,
                attributes: Vec::new(),
            });
        } else {
            self.latest_token = Some(HtmlToken::EndTag { tag: String::new() });
        }
    }
    fn reconsume_input(&mut self) -> char {
        self.reconsume = false;
        self.input[self.pos - 1]
    }
    fn append_tag_name(&mut self, c: char) {
        assert!(self.latest_token.is_some());
        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    ref mut tag,
                    self_closing: _,
                    attributes: _,
                }
                | HtmlToken::EndTag { ref mut tag } => tag.push(c),
                _ => panic!("`latest_token` should be either StartTag or EndTag"),
            }
        }
    }
    fn take_latest_token(&mut self) -> Option<HtmlToken> {
        assert!(self.latest_token.is_some());
        let t = self.latest_token.as_ref().cloned();
        self.latest_token = None;
        assert!(self.latest_token.is_none());
        t
    }
    fn start_new_attribute(&mut self) {
        assert!(self.latest_token.is_some());
        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    tag: _,
                    self_closing: _,
                    ref mut attributes,
                } => {
                    attributes.push(Attribute::new());
                }
                _ => panic!("`latest_token` should be either StartTag"),
            }
        }
    }
    fn append_attribute(&mut self, c: char, is_name: bool) {
        assert!(self.latest_token.is_some());
        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    tag: _,
                    self_closing: _,
                    ref mut attributes,
                } => {
                    let len = attributes.len();
                    assert!(len > 0);
                    attributes[len - 1].add_char(c, is_name);
                }
                _ => panic!("`latest_token` should be either StartTag"),
            }
        }
    }
    fn set_self_closing_flag(&mut self) {
        assert!(self.latest_token.is_some());
        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    tag: _,
                    ref mut self_closing,
                    attributes: _,
                } => *self_closing = true,
                _ => panic!("`latest_token` should be eitherStartTag"),
            }
        }
    }
}

impl Iterator for HtmlTokenizer {
    type Item = HtmlToken;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }
        loop {
            let c = match self.reconsume {
                true => self.reconsume_input(),
                false => self.consume_next_input(),
            };

            match self.state {
                State::Data => {
                    if c == '<' {
                        self.state = State::TagOpen;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::TagOpen => {
                    if c == '/' {
                        // ── ❶
                        self.state = State::EndTagOpen;
                        continue;
                    }
                    if c.is_ascii_alphabetic() {
                        // ── ❷
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(true);
                        continue;
                    }
                    if self.is_eof() {
                        // ── ❸
                        return Some(HtmlToken::Eof);
                    }
                    self.reconsume = true;
                    self.state = State::Data;
                }
                State::EndTagOpen => {
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    if c.is_ascii_alphabetic() {
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(false);
                        continue;
                    }
                }
                State::TagName => {
                    if c == ' ' {
                        // ── ❶
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        // ── ❷
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        // ── ❸
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_uppercase() {
                        // ── ❹
                        self.append_tag_name(c.to_ascii_lowercase());
                        continue;
                    }
                    if self.is_eof() {
                        // ── ❺
                        return Some(HtmlToken::Eof);
                    }
                    self.append_tag_name(c);
                }
                State::BeforeAttributeName => {
                    if c == '/' || c == '>' || self.is_eof() {
                        // ── ❶
                        self.reconsume = true;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::AttributeName => {
                    if c == ' ' || c == '/' || c == '>' || self.is_eof() {
                        // ── ❶
                        self.reconsume = true;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    if c == '=' {
                        // ── ❷
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    if c.is_ascii_uppercase() {
                        self.append_attribute(c.to_ascii_lowercase(), /*is_name*/ true);
                        continue;
                    }
                    self.append_attribute(c, /*is_name*/ true);
                }
                State::AfterAttributeName => {
                    if c == ' ' {
                        // 空白文字は無視する
                        continue;
                    }
                    if c == '/' {
                        // ── ❶
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '=' {
                        // ── ❷
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    if c == '>' {
                        // ── ❸
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        // ── ❹
                        return Some(HtmlToken::Eof);
                    }
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::BeforeAttributeValue => {
                    if c == ' ' {
                        // 空白文字は無視する
                        continue;
                    }
                    if c == '"' {
                        // ── ❶
                        self.state = State::AttributeValueDoubleQuoted;
                        continue;
                    }
                    if c == '\'' {
                        // ── ❷
                        self.state = State::AttributeValueSingleQuoted;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::AttributeValueUnquoted;
                }
                State::AttributeValueDoubleQuoted => {
                    if c == '"' {
                        // ── ❶
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        // ── ❷
                        return Some(HtmlToken::Eof);
                    }
                    self.append_attribute(c, /*is_name*/ false);
                }
                State::AttributeValueSingleQuoted => {
                    if c == '\'' {
                        // ── ❶
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        // ── ❷
                        return Some(HtmlToken::Eof);
                    }
                    self.append_attribute(c, /*is_name*/ false);
                }
                State::AttributeValueUnquoted => {
                    if c == ' ' {
                        // ── ❶
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '>' {
                        // ── ❷
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        // ── ❸
                        return Some(HtmlToken::Eof);
                    }
                    self.append_attribute(c, /*is_name*/ false);
                }
                State::AfterAttributeValueQuoted => {
                    if c == ' ' {
                        // ── ❶
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        // ── ❷
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        // ── ❸
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        // ── ❹
                        return Some(HtmlToken::Eof);
                    }
                    self.reconsume = true;
                    self.state = State::BeforeAttributeValue;
                }
                State::SelfClosingStartTag => {
                    if c == '>' {
                        // ── ❶
                        self.set_self_closing_flag();
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        // ── ❷
                        // invalid parse error.
                        return Some(HtmlToken::Eof);
                    }
                }
                State::ScriptData => {
                    if c == '<' {
                        // ── ❶
                        self.state = State::ScriptDataLessThanSign;
                        continue;
                    }
                    if self.is_eof() {
                        // ── ❷
                        return Some(HtmlToken::Eof);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::ScriptDataLessThanSign => {
                    if c == '/' {
                        // ── ❶
                        // 一時的なバッファを空文字でリセットする
                        self.buf = String::new();
                        self.state = State::ScriptDataEndTagOpen;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::ScriptData;
                    return Some(HtmlToken::Char('<'));
                }
                State::ScriptDataEndTagOpen => {
                    if c.is_ascii_alphabetic() {
                        // ── ❶
                        self.reconsume = true;
                        self.state = State::ScriptDataEndTagName;
                        self.create_tag(false);
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::ScriptData;
                    // 仕様では、"<" と "/" の 2 つの文字トークンを返すとなっているが、
                    // 私たちの実装では next メソッドからは一つのトークンしか返せない
                    // ため、"<" のトークンのみを返す
                    return Some(HtmlToken::Char('<'));
                }
                State::ScriptDataEndTagName => {
                    if c == '>' {
                        // ── ❶
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_alphabetic() {
                        // ── ❷
                        self.buf.push(c);
                        self.append_tag_name(c.to_ascii_lowercase());
                        continue;
                    }
                    self.state = State::TemporaryBuffer;
                    self.buf = String::from("</") + &self.buf;
                    self.buf.push(c);
                    continue;
                }
                State::TemporaryBuffer => {
                    self.reconsume = true;
                    if self.buf.chars().count() == 0 {
                        self.state = State::ScriptData;
                        continue;
                    }
                    // 最初の 1 文字を削除する
                    let c = self
                        .buf
                        .chars()
                        .nth(0)
                        .expect("self.buf should have at least 1 char");
                    self.buf.remove(0);
                    return Some(HtmlToken::Char(c));
                }
            }
        }
    }
}
