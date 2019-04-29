//! Implementations of SRE commands.
use super::*;
use std::io::Write;
use std::str::FromStr;

fn p(w: &mut Write, s: &str) -> std::io::Result<()> {
    write!(w, "{}", s)
}

#[derive(Debug, PartialEq)]
pub struct P;

impl<'a> SimpleCommand<'a> for P {
    fn execute(&self, w: &mut Write, buffer: &mut Buffer, dot: Range) -> Result<Range, Box<Error>> {
        p(w, &buffer.data[dot.0..dot.1])?;

        Ok(dot)
    }
    fn to_tuple(&self) -> (char, LinkedList<String>) {
        ('p', LinkedList::new())
    }
}

#[derive(Debug, PartialEq)]
pub struct A(pub String);

impl<'a> SimpleCommand<'a> for A {
    fn execute(
        &self,
        _w: &mut Write,
        buffer: &mut Buffer,
        dot: Range,
    ) -> Result<Range, Box<Error>> {
        buffer.change(dot, true, &self.0);

        Ok(Range(dot.1, dot.1 + self.0.len()))
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        let mut list = LinkedList::new();
        list.push_back(self.0.clone());
        ('a', list)
    }
}

#[derive(Debug, PartialEq)]
pub struct C(pub String);

impl<'a> SimpleCommand<'a> for C {
    fn execute(
        &self,
        _w: &mut Write,
        buffer: &mut Buffer,
        dot: Range,
    ) -> Result<Range, Box<Error>> {
        buffer.change(dot, false, &self.0);

        Ok(Range(dot.0, dot.0 + self.0.len()))
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        let mut list = LinkedList::new();
        list.push_back(self.0.clone());
        ('c', list)
    }
}

#[derive(Debug, PartialEq)]
pub struct I(pub String);

impl<'a> SimpleCommand<'a> for I {
    fn execute(
        &self,
        _w: &mut Write,
        buffer: &mut Buffer,
        dot: Range,
    ) -> Result<Range, Box<Error>> {
        let mut replacement = String::from_str(&self.0).unwrap();
        replacement.push_str(&buffer.data[dot.0..dot.1]);
        buffer.change(dot, false, &replacement);

        Ok(Range(dot.0, dot.0 + self.0.len()))
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        let mut list = LinkedList::new();
        list.push_back(self.0.clone());
        ('c', list)
    }
}

#[derive(Debug, PartialEq)]
pub struct D;

impl<'a> SimpleCommand<'a> for D {
    fn execute(
        &self,
        _w: &mut Write,
        buffer: &mut Buffer,
        dot: Range,
    ) -> Result<Range, Box<Error>> {
        buffer.change(dot, false, "");

        Ok(Range(dot.0, dot.0))
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        ('c', LinkedList::new())
    }
}

#[derive(Debug)]
pub struct X(pub String, pub SRECommand);

impl<'a> SimpleCommand<'a> for X {
    fn execute(&self, w: &mut Write, buffer: &mut Buffer, dot: Range) -> Result<Range, Box<Error>> {
        let re = regex::Regex::new(&self.0)?;
        let mut addresses = Vec::new();
        for m in re.find_iter(&buffer.data[dot.0..dot.1]) {
            addresses.push(Range(m.start(), m.end()));
        }
        let mut last: Option<Range> = None;
        for addr in addresses {
            let iv = Invocation::new(self.1.clone(), buffer, Some(addr))?;
            last = Some(iv.execute(w, buffer)?);
        }
        Ok(last.unwrap_or(dot))
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        let mut list = LinkedList::new();
        list.push_back(self.0.clone());
        ('a', list)
    }
}

#[derive(Debug)]
pub struct Equals;

impl<'a> SimpleCommand<'a> for Equals {
    fn execute(
        &self,
        w: &mut Write,
        _buffer: &mut Buffer,
        dot: Range,
    ) -> Result<Range, Box<Error>> {
        writeln!(w, "#{},#{}", dot.0, dot.1)?;
        Ok(dot)
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        ('=', LinkedList::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::sre::SimpleCommand;
    #[test]
    fn smoke() {
        let mut b = super::Buffer::new("xd lol".as_bytes()).unwrap();
        let addr = b.new_address(0, 2).range();
        let p = super::P;
        let mut w = Vec::new();
        p.execute(&mut w, &mut b, addr).unwrap();
        assert_eq!(String::from_utf8_lossy(&w[..]), "xd");
    }
}
