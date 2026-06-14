use std::sync::atomic::{AtomicU32, Ordering};

/// 迭代预算控制（线程安全）
#[derive(Debug)]
pub struct IterationBudget {
    remaining: AtomicU32,
    max_total: u32,
    refund_count: AtomicU32,
}

impl IterationBudget {
    pub fn new(max_total: u32) -> Self {
        Self {
            remaining: AtomicU32::new(max_total),
            max_total,
            refund_count: AtomicU32::new(0),
        }
    }

    /// 消耗一次迭代，返回 false 表示预算耗尽
    pub fn consume(&self) -> bool {
        let mut current = self.remaining.load(Ordering::Acquire);
        loop {
            if current == 0 {
                return false;
            }
            match self.remaining.compare_exchange(
                current,
                current - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(updated) => current = updated,
            }
        }
    }

    /// 退还一次迭代（如代码执行或压缩后）
    pub fn refund(&self) {
        let mut current = self.remaining.load(Ordering::Acquire);
        loop {
            if current >= self.max_total {
                return;
            }
            match self.remaining.compare_exchange(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.refund_count.fetch_add(1, Ordering::Release);
                    return;
                }
                Err(updated) => current = updated,
            }
        }
    }

    /// 剩余迭代次数
    pub fn remaining(&self) -> u32 {
        self.remaining.load(Ordering::Acquire)
    }

    /// 总退还次数
    pub fn total_refunds(&self) -> u32 {
        self.refund_count.load(Ordering::Acquire)
    }

    /// 是否耗尽
    pub fn is_exhausted(&self) -> bool {
        self.remaining.load(Ordering::Acquire) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_creation() {
        let b = IterationBudget::new(10);
        assert_eq!(b.remaining(), 10);
        assert!(!b.is_exhausted());
    }

    #[test]
    fn test_consume() {
        let b = IterationBudget::new(3);
        assert!(b.consume());
        assert!(b.consume());
        assert!(b.consume());
        assert!(!b.consume()); // 用尽
        assert!(b.is_exhausted());
    }

    #[test]
    fn test_refund() {
        let b = IterationBudget::new(3);
        b.consume();
        b.consume();
        b.refund();
        assert_eq!(b.remaining(), 2);
        assert_eq!(b.total_refunds(), 1);
    }

    #[test]
    fn test_refund_no_overflow() {
        let b = IterationBudget::new(3);
        b.refund(); // 不应超过 max_total
        assert_eq!(b.remaining(), 3);
    }
}
