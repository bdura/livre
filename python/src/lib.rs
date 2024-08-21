mod page;

use page::PdfElement;
use pyo3::prelude::*;

#[pymodule]
fn livre_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[pyfn(m)]
    fn read_page(path: &str, page: usize) -> Vec<PdfElement> {
        page::read_page(path, page)
    }

    Ok(())
}
