use cluster_math::penalty::compute_penalty;
use cluster_math::FPDecimal;
use plotters::prelude::*;
use std::str::FromStr;

fn f32_penalty(score: f32, a_pos: f32, s_pos: f32, a_neg: f32, s_neg: f32) -> f32 {
    if score < 0f32 {
        1f32 - a_neg * (score / s_neg).tanh()
    } else {
        1f32 - a_pos * (score / s_pos).tanh()
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a_pos = FPDecimal::from_str("1")?;
    let s_pos = FPDecimal::from_str("1")?;
    let a_neg = FPDecimal::from_str("0.005")?;
    let s_neg = FPDecimal::from_str("0.5")?;

    let root = BitMapBackend::new("plot.png", (640 * 2, 480 * 2)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "penalty function error benchmark",
            ("sans-serif", 20).into_font(),
        )
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(-2f32..2f32, 0f32..1.2f32)?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            (-2000..=2000).map(|x| x as f32 / 1000.0).map(|x| {
                let native_penalty =
                    f32_penalty(x, a_pos.into(), s_pos.into(), a_neg.into(), s_neg.into());
                (x, native_penalty)
            }),
            &BLUE,
        ))?
        .label("f32 penalty")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart
        .draw_series(LineSeries::new(
            (-2000..=2000).map(|x| x as f32 / 1000.0).map(|x| {
                let fp = FPDecimal::from_str(&x.to_string()).unwrap();
                let fp_penalty: f32 = compute_penalty(fp, a_pos, s_pos, a_neg, s_neg).into();
                (x, fp_penalty)
            }),
            &RED,
        ))?
        .label("FPDecimal penalty")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .draw_series(LineSeries::new(
            (-2000..=2000).map(|x| x as f32 / 1000.0).map(|x| {
                let fp = FPDecimal::from_str(&x.to_string()).unwrap();
                let native_penalty =
                    f32_penalty(x, a_pos.into(), s_pos.into(), a_neg.into(), s_neg.into());
                let fp_penalty: f32 = compute_penalty(fp, a_pos, s_pos, a_neg, s_neg).into();
                let err = (native_penalty - fp_penalty).abs();
                (x, err * 1_000_000f32)
            }),
            &GREEN,
        ))?
        .label("err * 10^6")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    Ok(())
}
