mod tests {
    use atomic_slice_pointer::OnceSlicePtr;
    use std::thread::scope;

    #[test]
    fn test_initialization() {
        let pointer = OnceSlicePtr::<u8>::new();
        let r_pointer = &pointer;
        let boxed = vec![1; 100].into_boxed_slice();
        scope(|s| {
            for i in 0..100 {
                s.spawn(move || {
                    if let Some(slice) = r_pointer.get() {
                        let _ = slice[i];
                    }
                });
            }
            s.spawn(|| {
                r_pointer.set(boxed).unwrap();
            });
        });
        let successful_read = scope(|s| s.spawn(|| r_pointer.get().is_some()).join().unwrap());
        assert!(successful_read);
    }
}
