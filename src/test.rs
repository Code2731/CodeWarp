/// Hello world 출력 함수
pub fn hello_world() {
    println!("Hello, world!");
}

/// 이름을 받아서 인사하는 함수
pub fn hello_name(name: &str) {
    println!("Hello, {}!", name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        // 이 테스트는 hello_world 함수가 패닉 없이 실행되는지만 확인합니다.
        // 실제 출력 확인은 integration test에서 수행하는 것이 좋습니다.
        hello_world();
        assert!(true); // 항상 통과
    }

    #[test]
    fn test_hello_name() {
        hello_name("Alice");
        hello_name("Bob");
        assert!(true); // 항상 통과
    }
}

fn main() {
    // 예시 사용법
    hello_world();
    hello_name("CodeWarp");
}