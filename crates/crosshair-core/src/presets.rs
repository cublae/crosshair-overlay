use crate::{Dot, Outline, Profile, Rgba, Shape};

/// Built-in profiles seeded into a fresh config and offered in the GUI.
pub fn all() -> Vec<(&'static str, Profile)> {
    vec![
        ("dot", dot()),
        ("cs-classic", cs_classic()),
        ("valorant", valorant()),
        ("t-shape", t_shape()),
        ("ring", ring()),
        ("sniper", sniper()),
    ]
}

pub fn get(name: &str) -> Option<Profile> {
    all().into_iter().find(|(n, _)| *n == name).map(|(_, p)| p)
}

pub fn names() -> Vec<&'static str> {
    all().into_iter().map(|(n, _)| n).collect()
}

fn dot() -> Profile {
    Profile {
        shape: Shape::Dot,
        color: Rgba::new(0.0, 1.0, 0.2, 1.0),
        dot: Dot {
            enabled: true,
            size: 2.0,
            color: None,
        },
        ..Profile::default()
    }
}

fn cs_classic() -> Profile {
    Profile {
        shape: Shape::Cross,
        color: Rgba::new(0.0, 1.0, 0.0, 1.0),
        thickness: 2.0,
        length: 9.0,
        gap: 5.0,
        ..Profile::default()
    }
}

fn valorant() -> Profile {
    Profile {
        shape: Shape::Cross,
        color: Rgba::new(0.0, 1.0, 1.0, 1.0),
        thickness: 3.0,
        length: 6.0,
        gap: 3.0,
        outline: Outline {
            enabled: true,
            width: 1.0,
            color: Rgba::new(0.0, 0.0, 0.0, 1.0),
        },
        dot: Dot {
            enabled: true,
            size: 1.5,
            color: None,
        },
        ..Profile::default()
    }
}

fn t_shape() -> Profile {
    Profile {
        shape: Shape::T,
        color: Rgba::new(1.0, 1.0, 1.0, 1.0),
        thickness: 2.0,
        length: 11.0,
        gap: 4.0,
        ..Profile::default()
    }
}

fn ring() -> Profile {
    Profile {
        shape: Shape::Circle,
        color: Rgba::new(1.0, 0.35, 0.0, 1.0),
        thickness: 1.5,
        length: 14.0,
        gap: 0.0,
        dot: Dot {
            enabled: true,
            size: 1.5,
            color: None,
        },
        ..Profile::default()
    }
}

fn sniper() -> Profile {
    Profile {
        shape: Shape::Cross,
        color: Rgba::new(1.0, 0.0, 0.0, 0.9),
        thickness: 1.0,
        length: 40.0,
        gap: 6.0,
        dot: Dot {
            enabled: true,
            size: 1.0,
            color: None,
        },
        ..Profile::default()
    }
}
