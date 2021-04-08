use std::rc::Rc;

use glib::ToValue;

#[test]
fn test_param_extraction() -> anyhow::Result<()> {
    macro_rules! signal {
        ($($param:expr),*) => {
            woab::Signal::new(
                Rc::new("signal".to_owned()),
                vec![$(
                    $param.to_value()
                ),*],
                ()
            );
        }
    }
    let () = signal!().params()?;
    match signal!(1i32, "two").params() {
        Err(woab::Error::NotAllParametersExtracted {
            signal,
            num_parameters,
            num_extracted,
        }) => {
            assert_eq!(signal, "signal");
            assert_eq!(num_parameters, 2);
            assert_eq!(num_extracted, 0);
        }
        Ok(woab::params!()) => {
            panic!("Should have failed");
        }
        Err(err) => Err(err)?,
    }

    let woab::params!(a: i32, b: String) = signal!(3i32, "four").params()?;
    assert_eq!(a, 3);
    assert_eq!(b, "four");

    let woab::params!(c: i32, _, d: f32) = signal!(5i32, "six", 7.0f32).params()?;
    assert_eq!(c, 5);
    assert_eq!(d, 7.0);
    Ok(())
}
