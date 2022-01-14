//! Development-related functionality

pub use blobby;

/// Define block cipher test
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "dev")))]
macro_rules! block_cipher_test {
    ($name:ident, $test_name:expr, $cipher:ty) => {
        #[test]
        fn $name() {
            use cipher::generic_array::{typenum::Unsigned, GenericArray};
            use cipher::{
                blobby::Blob3Iterator, BlockDecryptMut, BlockEncryptMut, BlockSizeUser, KeyInit,
            };

            fn run_test(key: &[u8], pt: &[u8], ct: &[u8]) -> bool {
                let mut state = <$cipher as KeyInit>::new_from_slice(key).unwrap();

                let mut block = GenericArray::clone_from_slice(pt);
                state.encrypt_block_mut(&mut block);
                if ct != block.as_slice() {
                    return false;
                }

                state.decrypt_block_mut(&mut block);
                if pt != block.as_slice() {
                    return false;
                }

                true
            }

            fn run_par_test(key: &[u8], pt: &[u8]) -> bool {
                type Block = cipher::Block<$cipher>;

                let mut state = <$cipher as KeyInit>::new_from_slice(key).unwrap();

                let block = Block::clone_from_slice(pt);
                let mut blocks1 = vec![block; 101];
                for (i, b) in blocks1.iter_mut().enumerate() {
                    *b = block;
                    b[0] = b[0].wrapping_add(i as u8);
                }
                let mut blocks2 = blocks1.clone();

                // check that `encrypt_blocks` and `encrypt_block`
                // result in the same ciphertext
                state.encrypt_blocks_mut(&mut blocks1);
                for b in blocks2.iter_mut() {
                    state.encrypt_block_mut(b);
                }
                if blocks1 != blocks2 {
                    return false;
                }

                // check that `encrypt_blocks` and `encrypt_block`
                // result in the same plaintext
                state.decrypt_blocks_mut(&mut blocks1);
                for b in blocks2.iter_mut() {
                    state.decrypt_block_mut(b);
                }
                if blocks1 != blocks2 {
                    return false;
                }

                true
            }

            let data = include_bytes!(concat!("data/", $test_name, ".blb"));
            for (i, row) in Blob3Iterator::new(data).unwrap().enumerate() {
                let [key, pt, ct] = row.unwrap();
                if !run_test(key, pt, ct) {
                    panic!(
                        "\n\
                         Failed test №{}\n\
                         key:\t{:?}\n\
                         plaintext:\t{:?}\n\
                         ciphertext:\t{:?}\n",
                        i, key, pt, ct,
                    );
                }

                // test parallel blocks encryption/decryption
                if !run_par_test(key, pt) {
                    panic!(
                        "\n\
                         Failed parallel test №{}\n\
                         key:\t{:?}\n\
                         plaintext:\t{:?}\n\
                         ciphertext:\t{:?}\n",
                        i, key, pt, ct,
                    );
                }
            }
        }
    };
}

/// Define `IvState` test
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "dev")))]
macro_rules! iv_state_test {
    ($name:ident, $cipher:ty, $method:ident $(,)?) => {
        #[test]
        fn $name() {
            use cipher::*;

            let mut blocks = [Block::<$cipher>::default(); 32];

            for (i, block) in blocks.iter_mut().enumerate() {
                for (j, b) in block.iter_mut().enumerate() {
                    *b = (i + j) as u8;
                }
            }

            let mut key = Key::<$cipher>::default();
            let mut iv = Iv::<$cipher>::default();
            key.iter_mut().for_each(|b| *b = 0x42);
            iv.iter_mut().for_each(|b| *b = 0x24);

            let mut cipher = <$cipher>::new(&key, &iv);
            let mut target = blocks.clone();
            cipher.$method(&mut target);

            for i in 0..32 {
                let mut blocks = blocks.clone();
                let (b1, b2) = blocks.split_at_mut(i);
                let mut cipher1 = <$cipher>::new(&key, &iv);
                cipher1.$method(b1);
                let temp_iv = cipher1.iv_state();
                let mut cipher2 = <$cipher>::new(&key, &temp_iv);
                cipher2.$method(b2);
                assert_eq!(blocks, target);
            }
        }
    };
}

/// Define block encryptor benchmark
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "dev")))]
macro_rules! block_encryptor_bench {
    (Key: $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        block_encryptor_bench!(
            {
                use $crate::KeyInit;
                let key = test::black_box(Default::default());
                <$cipher>::new(&key)
            },
            $cipher,
            $block_name,
            $blocks_name,
        );
    };
    (KeyIv: $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        block_encryptor_bench!(
            {
                use $crate::KeyIvInit;
                let key = test::black_box(Default::default());
                let iv = test::black_box(Default::default());
                <$cipher>::new(&key, &iv)
            },
            $cipher,
            $block_name,
            $blocks_name,
        );
    };
    ($init:block, $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        #[bench]
        pub fn $block_name(bh: &mut test::Bencher) {
            use cipher::BlockEncryptMut;

            let mut cipher = $init;
            let mut blocks = vec![Default::default(); 16 * 1024];

            bh.iter(|| {
                for block in blocks.iter_mut() {
                    cipher.encrypt_block_mut(block);
                }
                test::black_box(&blocks);
            });
            bh.bytes = (blocks.len() * blocks[0].len()) as u64;
        }

        #[bench]
        pub fn $blocks_name(bh: &mut test::Bencher) {
            use cipher::BlockEncryptMut;

            let mut cipher = $init;
            let mut blocks = vec![Default::default(); 16 * 1024];

            bh.iter(|| {
                cipher.encrypt_blocks_mut(&mut blocks);
                test::black_box(&blocks);
            });
            bh.bytes = (blocks.len() * blocks[0].len()) as u64;
        }
    };
}

/// Define block decryptor benchmark
#[macro_export]
#[cfg_attr(docsrs, doc(cfg(feature = "dev")))]
macro_rules! block_decryptor_bench {
    (Key: $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        block_decryptor_bench!(
            {
                use $crate::KeyInit;
                let key = test::black_box(Default::default());
                <$cipher>::new(&key)
            },
            $cipher,
            $block_name,
            $blocks_name,
        );
    };
    (KeyIv: $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        block_decryptor_bench!(
            {
                use $crate::KeyIvInit;
                let key = test::black_box(Default::default());
                let iv = test::black_box(Default::default());
                <$cipher>::new(&key, &iv)
            },
            $cipher,
            $block_name,
            $blocks_name,
        );
    };
    ($init:block, $cipher:ty, $block_name:ident, $blocks_name:ident $(,)? ) => {
        #[bench]
        pub fn $block_name(bh: &mut test::Bencher) {
            use cipher::BlockDecryptMut;

            let mut cipher = $init;
            let mut blocks = vec![Default::default(); 16 * 1024];

            bh.iter(|| {
                for block in blocks.iter_mut() {
                    cipher.decrypt_block_mut(block);
                }
                test::black_box(&blocks);
            });
            bh.bytes = (blocks.len() * blocks[0].len()) as u64;
        }

        #[bench]
        pub fn $blocks_name(bh: &mut test::Bencher) {
            use cipher::BlockDecryptMut;

            let mut cipher = $init;
            let mut blocks = vec![Default::default(); 16 * 1024];

            bh.iter(|| {
                cipher.decrypt_blocks_mut(&mut blocks);
                test::black_box(&blocks);
            });
            bh.bytes = (blocks.len() * blocks[0].len()) as u64;
        }
    };
}
