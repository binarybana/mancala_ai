#![feature(test)]
extern crate test;
use test::Bencher;
// extern crate mancala_ai;

const SIZE: usize = 1024;

#[bench]
fn alloc(bench: &mut Bencher) {
    bench.iter(|| (0..SIZE).map(|_| 0u8).collect::<Vec<_>>())
}
