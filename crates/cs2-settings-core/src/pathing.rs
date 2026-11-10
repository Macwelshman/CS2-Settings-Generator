use std::path::{Component, Path};

pub fn portable_relative_path(from_directory: &Path, destination: &Path) -> Option<String> {
    let from = from_directory.components().collect::<Vec<_>>();
    let to = destination.components().collect::<Vec<_>>();

    let mut common = 0;
    while common < from.len() && common < to.len() && components_equal(from[common], to[common]) {
        common += 1;
    }

    if common == 0 && (from.first()?.as_os_str() != to.first()?.as_os_str()) {
        return None;
    }

    let mut parts = Vec::new();
    for component in &from[common..] {
        if matches!(component, Component::Normal(_)) {
            parts.push("..".to_owned());
        }
    }
    for component in &to[common..] {
        if let Component::Normal(value) = component {
            parts.push(value.to_string_lossy().into_owned());
        }
    }

    Some(parts.join("/"))
}

fn components_equal(left: Component<'_>, right: Component<'_>) -> bool {
    left.as_os_str() == right.as_os_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_portable_sibling_path() {
        let source = Path::new("/Export/Tower");
        let destination = Path::new("/Export/Main/Main_BaseColor.png");
        assert_eq!(
            portable_relative_path(source, destination).as_deref(),
            Some("../Main/Main_BaseColor.png")
        );
    }
}
