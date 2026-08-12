use smart_bc::voice::listener::RingBuffer;

#[test]
fn ringbuffer_keeps_latest() {
    let mut rb = RingBuffer::new(2, 10); // 2s @ 10Hz = 20 采样
    rb.push(&[1.0f32; 10]);
    rb.push(&[2.0f32; 10]);
    rb.push(&[3.0f32; 10]); // 溢出最早 10 个
    let snap = rb.snapshot();
    assert_eq!(snap.len(), 20);
    assert!(snap.iter().all(|&s| s == 2.0 || s == 3.0));
}

#[test]
fn ringbuffer_clear() {
    let mut rb = RingBuffer::new(1, 10);
    rb.push(&[1.0f32; 10]);
    rb.clear();
    assert!(rb.snapshot().is_empty());
}
