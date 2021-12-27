use std::{collections::BTreeMap, error::Error, path::Path};
type Param1 = f32;
type Param2 = (f32, f32);

type VecParam1 = Vec<Param1>;
type VecParam2 = Vec<Param2>;

#[derive(Debug, Copy, Clone)]
pub enum CommandType {
    Absolute,
    Relative,
}

#[derive(Debug, Clone)]
pub enum SvgDraw {
    MoveTo(CommandType, Param2),
    LineTo(CommandType, Param2),
    HorizontalTo(CommandType, Param1),
    VerticalTo(CommandType, Param1),
    EndPath,
    ClosePath,
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
fn ws<'a, F: 'a, O, E: nom::error::ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> nom::IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> nom::IResult<&'a str, O, E>,
{
    nom::sequence::delimited(
        nom::character::complete::multispace0,
        inner,
        nom::character::complete::multispace0,
    )
}

fn ws_float(input: &str) -> nom::IResult<&str, f32> {
    ws(nom::number::complete::float)(input)
}

pub fn param1(input: &str) -> nom::IResult<&str, Param1> {
    ws_float(input)
}

pub fn vec_param1(input: &str) -> nom::IResult<&str, VecParam1> {
    nom::multi::many1(param1)(input)
}

pub fn param2(input: &str) -> nom::IResult<&str, Param2> {
    let (input, lhs) = nom::number::complete::float(input)?;
    let (input, _) = nom::bytes::complete::tag(",")(input)?;
    let (input, rhs) = nom::number::complete::float(input)?;
    Ok((input, (lhs, rhs)))
}

pub fn vec_param2(input: &str) -> nom::IResult<&str, VecParam2> {
    nom::multi::many1(ws(param2))(input)
}

pub fn move_to_absolute(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("M")(input)?;
    let (input, output) = vec_param2(input)?;

    let mut iter = output.into_iter();

    let head = SvgDraw::MoveTo(CommandType::Absolute, iter.next().unwrap());
    let rem = iter.map(|output| SvgDraw::LineTo(CommandType::Absolute, output));

    let output = std::iter::once(head).chain(rem).collect();

    Ok((input, output))
}

pub fn move_to_relative(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("m")(input)?;
    let (input, output) = vec_param2(input)?;

    let mut iter = output.into_iter();

    let head = SvgDraw::MoveTo(CommandType::Relative, iter.next().unwrap());
    let rem = iter.map(|output| SvgDraw::LineTo(CommandType::Relative, output));

    let output = std::iter::once(head).chain(rem).collect();

    Ok((input, output))
}

pub fn move_to(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, output) = nom::branch::alt((move_to_absolute, move_to_relative))(input)?;
    Ok((input, output))
}

pub fn line_to_absolute(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("L")(input)?;
    let (input, output) = vec_param2(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::LineTo(CommandType::Absolute, output))
        .collect();

    Ok((input, output))
}

pub fn line_to_relative(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("l")(input)?;
    let (input, output) = vec_param2(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::LineTo(CommandType::Relative, output))
        .collect();

    Ok((input, output))
}

pub fn horizontal_to_absolute(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("H")(input)?;
    let (input, output) = vec_param1(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::HorizontalTo(CommandType::Absolute, output))
        .collect();

    Ok((input, output))
}

pub fn horizontal_to_relative(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("h")(input)?;
    let (input, output) = vec_param1(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::HorizontalTo(CommandType::Relative, output))
        .collect();

    Ok((input, output))
}

pub fn vertical_to_absolute(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("V")(input)?;
    let (input, output) = vec_param1(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::VerticalTo(CommandType::Absolute, output))
        .collect();

    Ok((input, output))
}

pub fn vertical_to_relative(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::bytes::complete::tag("v")(input)?;
    let (input, output) = vec_param1(input)?;

    let output = output
        .into_iter()
        .map(|output| SvgDraw::VerticalTo(CommandType::Relative, output))
        .collect();

    Ok((input, output))
}

pub fn close_path(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, _) = nom::branch::alt((
        nom::bytes::complete::tag("Z"),
        nom::bytes::complete::tag("z"),
    ))(input)?;

    Ok((input, vec![SvgDraw::ClosePath]))
}

pub fn svg_draw_variant(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    nom::branch::alt((
        move_to_absolute,
        move_to_relative,
        line_to_absolute,
        line_to_relative,
        horizontal_to_absolute,
        horizontal_to_relative,
        vertical_to_absolute,
        vertical_to_relative,
        close_path,
    ))(input)
}

pub fn svg_draw(input: &str) -> nom::IResult<&str, Vec<SvgDraw>> {
    let (input, output) = nom::multi::many1(svg_draw_variant)(input)?;

    Ok((input, output.into_iter().flatten().collect()))
}

#[derive(Debug, Default, Clone)]
pub struct SvgLayers(BTreeMap<String, BTreeMap<String, Vec<SvgDraw>>>);

impl SvgLayers {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<SvgLayers, Box<dyn Error>> {
        let mut group_stack = vec![];
        let mut layers = SvgLayers::default();

        let mut buf = String::new();
        for event in svg::open(path, &mut buf)? {
            match event {
                svg::parser::Event::Tag(path, tag_type, attributes) => match path {
                    "g" => match tag_type {
                        svg::node::element::tag::Type::Start => {
                            let value = attributes.get("inkscape:label").unwrap();
                            group_stack.push(value.to_string());
                        }
                        svg::node::element::tag::Type::End => {
                            group_stack.pop();
                        }
                        _ => (),
                    },
                    "path" => {
                        let value = attributes.get("d").unwrap().to_string();
                        let (_, svg_draw) = svg_draw(&value).unwrap();
                        layers
                            .0
                            .entry(group_stack[0].clone())
                            .or_default()
                            .entry(group_stack[1].clone())
                            .or_default()
                            .extend(svg_draw.into_iter().chain(std::iter::once(SvgDraw::EndPath)));
                    }
                    _ => (),
                },
                _ => {}
            }
        }

        Ok(layers)
    }

    pub fn commands(&self) -> &BTreeMap<String, BTreeMap<String, Vec<SvgDraw>>> {
        &self.0
    }

    pub fn meshes(&self) -> SvgMeshes {
        self.0
            .iter()
            .map(|(category, layers)| {
                let layers = layers
                    .iter()
                    .map(|(grapheme, commands)| {
                        let mut vertices = vec![];
                        let mut indices = vec![];

                        let mut cur_pos = (0.0f32, 0.0f32);
                        let mut index = 0;
                        let mut start_index = index;
                        for command in commands {
                            match command {
                                SvgDraw::MoveTo(command_type, in_position) => {
                                    match command_type {
                                        CommandType::Absolute => cur_pos = *in_position,
                                        CommandType::Relative => {
                                            let (x, y) = cur_pos;
                                            let (in_x, in_y) = *in_position;
                                            cur_pos = (x + in_x, y + in_y)
                                        }
                                    };
                                    start_index = index;

                                    vertices.push(cur_pos);
                                }
                                SvgDraw::LineTo(command_type, in_position) => {
                                    match command_type {
                                        CommandType::Absolute => cur_pos = *in_position,
                                        CommandType::Relative => {
                                            let (x, y) = cur_pos;
                                            let (in_x, in_y) = *in_position;
                                            cur_pos = (x + in_x, y + in_y)
                                        }
                                    };
                                    vertices.push(cur_pos);

                                    indices.push(index);
                                    index += 1;
                                    indices.push(index);
                                }
                                SvgDraw::HorizontalTo(command_type, in_x) => {
                                    let (x, y) = cur_pos;
                                    match command_type {
                                        CommandType::Absolute => cur_pos = (*in_x, y),
                                        CommandType::Relative => cur_pos = (x + *in_x, y),
                                    };
                                    vertices.push(cur_pos);

                                    indices.push(index);
                                    index += 1;
                                    indices.push(index);
                                }
                                SvgDraw::VerticalTo(command_type, in_y) => {
                                    let (x, y) = cur_pos;
                                    match command_type {
                                        CommandType::Absolute => cur_pos = (x, *in_y),
                                        CommandType::Relative => cur_pos = (x, y + *in_y),
                                    };
                                    vertices.push(cur_pos);

                                    indices.push(index);
                                    index += 1;
                                    indices.push(index);
                                }
                                SvgDraw::EndPath => {
                                    cur_pos = (0.0, 0.0);
                                    index += 1;
                                }
                                SvgDraw::ClosePath => {
                                    indices.push(index);
                                    indices.push(start_index);
                                }
                            }
                        }

                        (grapheme.clone(), (vertices, indices))
                    })
                    .collect();
                (category.clone(), layers)
            })
            .collect()
    }
}

pub type SvgMeshes = BTreeMap<String, BTreeMap<String, (Vec<(f32, f32)>, Vec<usize>)>>;
// Aa
// Ee
