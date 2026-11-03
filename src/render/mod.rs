//! 렌더링 계층. `Greeting`을 ANSI 문자열로 바꾸는 순수 함수만 둔다.

pub mod color;
pub mod text;

/// 색 출력 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    /// 색 없음. 이스케이프 시퀀스를 하나도 쓰지 않는다.
    None,
}
