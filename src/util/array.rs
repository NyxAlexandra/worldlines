use std::mem::MaybeUninit;

pub fn try_map<const N: usize, T, U, E>(
    array: [T; N],
    mut f: impl FnMut(T) -> Result<U, E>,
) -> Result<[U; N], E> {
    let mut out = MaybeUninit::<[U; N]>::uninit();
    let ptr = out.as_mut_ptr().cast::<U>();

    for (i, value) in array.into_iter().enumerate() {
        let mapped = f(value)?;

        unsafe {
            ptr.add(i).write(mapped);
        }
    }

    Ok(unsafe { out.assume_init() })
}

pub unsafe fn get_many_unchecked_mut<T, const N: usize>(
    this: &mut [T],
    indices: [usize; N],
) -> [&mut T; N] {
    // adapted from the standard library

    let slice: *mut [T] = this;
    let mut arr: MaybeUninit<[&mut T; N]> = MaybeUninit::uninit();
    let arr_ptr = arr.as_mut_ptr();

    unsafe {
        for i in 0..N {
            let index = *indices.get_unchecked(i);

            *get_unchecked_mut(arr_ptr, i) =
                &mut *get_unchecked_mut(slice, index);
        }
        arr.assume_init()
    }
}

unsafe fn get_unchecked_mut<T>(this: *mut [T], index: usize) -> *mut T {
    let ptr: *mut T = this as _;

    unsafe { ptr.add(index) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_try_map() {
        assert_eq!(
            try_map(["1", "2", "3"], |v| v.parse::<u32>()).unwrap(),
            [1, 2, 3]
        );
        assert!(try_map(["1", "2a", "3"], |v| v.parse::<u32>()).is_err());
    }

    #[test]
    fn array_get_many_unchecked_mut() {
        let x = &mut [1, 2, 4];

        unsafe {
            let [a, b] = get_many_unchecked_mut(x, [0, 2]);

            *a *= 10;
            *b *= 100;
        }

        assert_eq!(x, &[10, 2, 400]);
    }
}
