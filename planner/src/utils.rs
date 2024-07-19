pub struct MyIntersperse<T, I> {
    iterator: I,
    sep: T,
    nxt: Option<T>,
}

pub trait MyIntersperseExt<T: Clone, I: Iterator>: Iterator<Item = T> {
    fn my_intersperse(self, sep: T) -> MyIntersperse<T, I>;
}

impl<T: Clone, I: Iterator<Item = T>> MyIntersperseExt<T, I> for I {
    fn my_intersperse(mut self, sep: T) -> MyIntersperse<T, I> {
        let next = self.next();
        MyIntersperse {
            iterator: self,
            sep: sep.clone(),
            nxt: next,
        }
    }
}

impl<T: Clone, I: Iterator<Item = T>> Iterator for MyIntersperse<T, I> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.nxt.take() {
            Some(item)
        } else {
            self.nxt = self.iterator.next();
            if self.nxt.is_some() {
                Some(self.sep.clone())
            } else {
                None
            }
        }
    }
}
