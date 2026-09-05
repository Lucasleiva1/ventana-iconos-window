use serde::Serialize;
use tauri::{Monitor, WebviewWindow};

use crate::model::{
    COLLAPSED_HEIGHT, DEFAULT_DRAWER_HEIGHT, DEFAULT_DRAWER_WIDTH, Drawer, MIN_DRAWER_HEIGHT,
    MIN_DRAWER_WIDTH,
};

const WORK_AREA_MARGIN_PHYSICAL: i32 = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub scale_factor: f64,
    pub position: Point,
    pub size: Dimensions,
    pub work_area: Area,
}

pub fn monitor_id(monitor: &Monitor) -> String {
    let name = monitor.name().map(String::as_str).unwrap_or("Monitor");
    format!("{name}@{},{}", monitor.position().x, monitor.position().y)
}

pub fn monitor_infos(window: &WebviewWindow) -> Result<Vec<MonitorInfo>, String> {
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("No se pudieron detectar los monitores: {error}"))?;
    let primary_id = window
        .primary_monitor()
        .map_err(|error| format!("No se pudo detectar el monitor principal: {error}"))?
        .as_ref()
        .map(monitor_id);

    Ok(monitors
        .iter()
        .map(|monitor| {
            let work_area = monitor.work_area();
            let id = monitor_id(monitor);
            MonitorInfo {
                primary: primary_id.as_ref() == Some(&id),
                id,
                name: monitor
                    .name()
                    .cloned()
                    .unwrap_or_else(|| "Monitor".to_owned()),
                scale_factor: monitor.scale_factor(),
                position: Point {
                    x: monitor.position().x,
                    y: monitor.position().y,
                },
                size: Dimensions {
                    width: monitor.size().width,
                    height: monitor.size().height,
                },
                work_area: Area {
                    x: work_area.position.x,
                    y: work_area.position.y,
                    width: work_area.size.width,
                    height: work_area.size.height,
                },
            }
        })
        .collect())
}

pub fn default_placement(
    window: &WebviewWindow,
    index: usize,
) -> Result<(i32, i32, String), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|error| format!("No se pudo detectar el monitor principal: {error}"))?
        .or_else(|| window.current_monitor().ok().flatten())
        .ok_or_else(|| "Windows no informó ningún monitor disponible".to_owned())?;
    let work_area = monitor.work_area();
    let offset = i32::try_from(index % 10).unwrap_or_default() * 28;
    let x = work_area.position.x + 42 + offset;
    let y = work_area.position.y + 42 + offset;
    Ok((x, y, monitor_id(&monitor)))
}

pub fn target_monitor<'a>(drawer: &Drawer, monitors: &'a [Monitor]) -> Option<&'a Monitor> {
    monitors
        .iter()
        .find(|monitor| monitor_id(monitor) == drawer.monitor_id)
        .or_else(|| {
            monitors.iter().find(|monitor| {
                let area = monitor.work_area();
                let right = i64::from(area.position.x) + i64::from(area.size.width);
                let bottom = i64::from(area.position.y) + i64::from(area.size.height);
                i64::from(drawer.x) >= i64::from(area.position.x)
                    && i64::from(drawer.x) < right
                    && i64::from(drawer.y) >= i64::from(area.position.y)
                    && i64::from(drawer.y) < bottom
            })
        })
        .or_else(|| monitors.first())
}

pub fn logical_limits(monitor: &Monitor) -> (f64, f64) {
    let scale = monitor.scale_factor().max(0.1);
    let work_area = monitor.work_area();
    let margin = f64::from(WORK_AREA_MARGIN_PHYSICAL * 2);
    let width = ((f64::from(work_area.size.width) - margin) / scale).max(MIN_DRAWER_WIDTH);
    let height = ((f64::from(work_area.size.height) - margin) / scale).max(MIN_DRAWER_HEIGHT);
    (width, height)
}

pub fn normalize_drawer(drawer: &mut Drawer, monitors: &[Monitor]) {
    let Some(monitor) = target_monitor(drawer, monitors) else {
        drawer.width = drawer.width.max(MIN_DRAWER_WIDTH);
        drawer.height = if drawer.collapsed {
            COLLAPSED_HEIGHT
        } else {
            drawer.height.max(MIN_DRAWER_HEIGHT)
        };
        return;
    };

    drawer.monitor_id = monitor_id(monitor);
    let (maximum_width, maximum_height) = logical_limits(monitor);

    drawer.expanded_width = drawer
        .expanded_width
        .max(MIN_DRAWER_WIDTH)
        .min(maximum_width);
    drawer.expanded_height = drawer
        .expanded_height
        .max(MIN_DRAWER_HEIGHT)
        .min(maximum_height);

    if drawer.collapsed {
        drawer.width = drawer.expanded_width;
        drawer.height = COLLAPSED_HEIGHT;
    } else {
        drawer.width = drawer.width.max(MIN_DRAWER_WIDTH).min(maximum_width);
        drawer.height = drawer.height.max(MIN_DRAWER_HEIGHT).min(maximum_height);
        drawer.expanded_width = drawer.width;
        drawer.expanded_height = drawer.height;
    }

    if !drawer.width.is_finite() {
        drawer.width = DEFAULT_DRAWER_WIDTH.min(maximum_width);
    }
    if !drawer.expanded_width.is_finite() {
        drawer.expanded_width = DEFAULT_DRAWER_WIDTH.min(maximum_width);
    }
    if !drawer.height.is_finite() {
        drawer.height = if drawer.collapsed {
            COLLAPSED_HEIGHT
        } else {
            DEFAULT_DRAWER_HEIGHT.min(maximum_height)
        };
    }
    if !drawer.expanded_height.is_finite() {
        drawer.expanded_height = DEFAULT_DRAWER_HEIGHT.min(maximum_height);
    }

    let area = monitor.work_area();
    let physical_width = (drawer.width * monitor.scale_factor()).round() as i64;
    let physical_height = (drawer.height * monitor.scale_factor()).round() as i64;
    let minimum_x = area.position.x + WORK_AREA_MARGIN_PHYSICAL;
    let minimum_y = area.position.y + WORK_AREA_MARGIN_PHYSICAL;
    let maximum_x = (i64::from(area.position.x) + i64::from(area.size.width)
        - physical_width
        - i64::from(WORK_AREA_MARGIN_PHYSICAL))
    .max(i64::from(minimum_x));
    let maximum_y = (i64::from(area.position.y) + i64::from(area.size.height)
        - physical_height
        - i64::from(WORK_AREA_MARGIN_PHYSICAL))
    .max(i64::from(minimum_y));

    drawer.x = i64::from(drawer.x).clamp(i64::from(minimum_x), maximum_x) as i32;
    drawer.y = i64::from(drawer.y).clamp(i64::from(minimum_y), maximum_y) as i32;
}
