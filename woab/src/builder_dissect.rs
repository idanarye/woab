use quick_xml::Reader;

pub fn dissect_builder_xml(buf_read: impl std::io::BufRead, targets: &mut [Vec<u8>], id_to_idx: impl Fn(&str) -> Option<usize>) -> Result<(), crate::Error> {
    let mut reader = Reader::from_reader(buf_read);
    let mut buf = Vec::new();

    struct WriteContext<'a> {
        writer: quick_xml::Writer<std::io::Cursor<&'a mut std::vec::Vec<u8>>>,
        prev_idx: Option<usize>,
        nesting: usize,
    }

    let mut contexts = targets.into_iter().map(|target| {
        WriteContext {
            writer: quick_xml::Writer::new(std::io::Cursor::new(target)),
            prev_idx: None,
            nesting: 0,
        }
    }).collect::<Vec<_>>();

    let mut context_idx = None;
    let mut current_nesting = 0;

    fn write_event(contexts: &mut [WriteContext], idx: Option<usize>, event: quick_xml::events::Event) -> Result<(), crate::Error> {
        if let Some(idx) = idx {
            contexts[idx].writer.write_event(event)?;
        } else {
            for context in contexts.iter_mut() {
                context.writer.write_event(event.clone())?;
            }
        }
        Ok(())
    }

    loop {
        match reader.read_event(&mut buf)? {
            quick_xml::events::Event::Start(e) => {
                current_nesting += 1;
                match e.name() {
                    b"object" => {
                        let new_idx = e.attributes().find_map(|attr| {
                            let attr = attr.ok()?;
                            if attr.key != b"id" {
                                return None;
                            }
                            let id = std::str::from_utf8(&attr.value).ok()?;
                            id_to_idx(id)
                        });
                        if let Some(new_idx) = new_idx {
                            contexts[new_idx].prev_idx = context_idx;
                            assert!(contexts[new_idx].nesting == 0);
                            contexts[new_idx].nesting = current_nesting;
                            context_idx = Some(new_idx);
                        }
                    }
                    _ => {}
                }
                write_event(&mut contexts, context_idx, quick_xml::events::Event::Start(e))?;
            },
            e @ quick_xml::events::Event::End(_) => {
                write_event(&mut contexts, context_idx, e)?;
                if let Some(idx) = context_idx {
                    if current_nesting == contexts[idx].nesting {
                        context_idx = contexts[idx].prev_idx;
                        contexts[idx].prev_idx = None;
                        contexts[idx].nesting = 0;
                    }
                }
                current_nesting -= 1;
            },
            quick_xml::events::Event::Eof => break,
            e => write_event(&mut contexts, context_idx, e)?,
        }
    }
    Ok(())
}
