use std::fmt;

// TODO can impl PartialEq, Eq
#[derive(Copy, Clone, Debug)]
enum Cell {
    Unknown,
    Empty,
    Wall,
    Monster,
    Chest,
}

#[derive(Debug)]
enum BoardError {
    // At least one cell is `Unsolved`
    Unsolved, // TODO do we need coordinates?
    WrongRowCount(Index),
    WrongColumnCount(Index),
    MonsterNotInDeadEnd(Index, Index),
    DeadEndWithNoMontster(Index, Index),
    NoTreasureRoomForChest(Index, Index),
    CorridorsTooWide(Index, Index),
    UnconnectedCorridors,
}

#[derive(Debug)]
struct ParseError; // TODO distinguish errors (but nobody actually cares)

type Index = i8;
const BOARD_SIZE: usize = 8;

#[derive(Debug)]
struct Board {
    cells: [[Cell; BOARD_SIZE]; BOARD_SIZE],
    column_counts: [u8; BOARD_SIZE],
    row_counts: [u8; BOARD_SIZE],
}

#[derive(Debug)]
struct Unsolvable;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            " {}",
            self.column_counts
                .into_iter()
                .map(|n| n.to_string())
                .collect::<String>()
        )?;
        for i in 0..BOARD_SIZE {
            writeln!(
                f,
                "{}{}",
                self.row_counts[i],
                self.cells[i]
                    .into_iter()
                    .map(|cell| match cell {
                        Cell::Unknown => ' ',
                        Cell::Empty => '.',
                        Cell::Wall => '#',
                        Cell::Monster => 'M',
                        Cell::Chest => 'C',
                    })
                    .collect::<String>()
            )?;
        }

        Ok(())
    }
}

impl Board {
    fn from_string(spec: &str) -> Result<Board, ParseError> {
        let mut lines = spec.lines().map(|s| s.as_bytes());
        let first_line = lines.next().ok_or(ParseError {})?;
        if first_line.len() != BOARD_SIZE + 1 {
            return Err(ParseError {});
        }
        let mut column_counts = [0u8; BOARD_SIZE];
        for i in 0..BOARD_SIZE {
            if !first_line[i + 1].is_ascii_digit() {
                return Err(ParseError {});
            }
            column_counts[i] = first_line[i + 1] - b'0';
        }
        let mut row_counts = [0u8; BOARD_SIZE];
        let mut cells = [[Cell::Empty; BOARD_SIZE]; BOARD_SIZE];
        for (i, line) in lines.enumerate() {
            if line.len() != BOARD_SIZE + 1 || !line[0].is_ascii_digit() {
                return Err(ParseError {});
            }
            row_counts[i] = line[0] - b'0';
            for j in 0..BOARD_SIZE {
                cells[i][j] = match line[j + 1] {
                    b' ' => Cell::Unknown,
                    b'.' => Cell::Empty,
                    b'#' => Cell::Wall,
                    b'M' => Cell::Monster,
                    b'C' => Cell::Chest,
                    _ => return Err(ParseError {}),
                }
            }
        }

        Ok(Board {
            cells,
            column_counts,
            row_counts,
        })
    }

    fn rows_acceptable(&self) -> Result<(), Index> {
        let unknown_counts = self
            .cells
            .iter()
            .map(|row| row.iter().filter(|c| matches!(c, Cell::Unknown)).count());
        let wall_counts = self
            .cells
            .iter()
            .map(|row| row.iter().filter(|c| matches!(c, Cell::Wall)).count());

        let ranges = wall_counts
            .zip(unknown_counts)
            .map(|(walls, unkns)| (walls as u8..=(walls + unkns) as u8));

        ranges
            .zip(self.row_counts)
            .enumerate()
            .find_map(|(i, (range, cnt))| (!range.contains(&cnt)).then_some(i))
            .map_or(Ok(()), |i| Err(i as Index))
    }

    fn cols_acceptable(&self) -> Result<(), Index> {
        let columns = (0..BOARD_SIZE).map(|i| self.cells.iter().map(move |row| row[i]));
        let wall_counts = columns
            .clone()
            .map(|col| col.filter(|c| matches!(c, Cell::Wall)).count() as u8);
        let unkn_counts =
            columns.map(|col| col.filter(|c| matches!(c, Cell::Unknown)).count() as u8);
        let ranges = wall_counts
            .zip(unkn_counts)
            .map(|(walls, unkns)| (walls..=walls + unkns));

        ranges
            .zip(self.column_counts)
            .enumerate()
            .find_map(|(i, (range, cnt))| (!range.contains(&cnt)).then_some(i))
            .map_or(Ok(()), |i| Err(i as Index))
    }

    fn check_solved(&self) -> Result<(), BoardError> {
        // * No `Unknown`
        // * All column & row counts are satisfied
        // * Monsters are in dead ends; all dead ends have monsters
        // * All treasure rooms have 3x3 empty space and one entrance
        // * No 2x2 empty spaces
        // * All corridors connected
        use BoardError as E;

        if self
            .cells
            .iter()
            .flatten()
            .any(|c| matches!(c, Cell::Unknown))
        {
            return Err(E::Unsolved);
        }

        let cur_row_counts = self
            .cells
            .iter()
            .map(|row| row.iter().filter(|c| matches!(c, Cell::Wall)).count() as u8);
        let bad_row = cur_row_counts
            .zip(self.row_counts)
            .enumerate()
            .find_map(|(i, (a, b))| (a != b).then_some(i));
        if let Some(r) = bad_row {
            return Err(E::WrongRowCount(r as Index));
        }

        let columns = (0..BOARD_SIZE).map(|i| self.cells.iter().map(move |row| row[i]));
        let cur_col_counts =
            columns.map(|col| col.filter(|c| matches!(c, Cell::Wall)).count() as u8);
        let bad_col = cur_col_counts
            .zip(self.column_counts)
            .enumerate()
            .find_map(|(i, (a, b))| (a != b).then_some(i));
        if let Some(c) = bad_col {
            return Err(E::WrongColumnCount(c as Index));
        }

        let mut treasure_rooms = Vec::<(Index, Index)>::new();

        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                let is_monster = matches!(self.cells[i][j], Cell::Monster);
                let is_dead_end = self.is_dead_end(i as Index, j as Index);
                if is_monster != is_dead_end {
                    // "if and only if" relation
                    return if is_monster {
                        Err(E::MonsterNotInDeadEnd(i as Index, j as Index))
                    } else {
                        Err(E::DeadEndWithNoMontster(i as Index, j as Index))
                    };
                }

                if let Cell::Chest = self.cells[i][j] {
                    let (r, c) = (i as Index, j as Index);
                    let treasure_room_candidates = [
                        (r - 2, c - 2),
                        (r - 2, c - 1),
                        (r - 2, c),
                        (r - 1, c - 2),
                        (r - 1, c - 1),
                        (r - 1, c),
                        (r, c - 2),
                        (r, c - 1),
                        (r, c),
                    ];
                    let maybe_room = treasure_room_candidates
                        .into_iter()
                        .find(|&(r, c)| self.is_treasure_room(r, c));
                    match maybe_room {
                        Some(room) => {
                            treasure_rooms.push(room);
                        }
                        None => {
                            return Err(E::NoTreasureRoomForChest(i as Index, j as Index));
                        }
                    }
                }
            }
        }

        let coords_to_check = {
            let mut check = [[true; BOARD_SIZE]; BOARD_SIZE];
            // . # # # # .
            // # # # # # #
            // # # # # # #
            // # # # # # #
            // # # # # # #
            // . # # # # .
            let affected_coords = (-1..=2)
                .map(|c| (-2, c)) // rect (-2, -1) ..= (-2, +2)
                .chain(
                    // rect (-1, -2) ..= (+2, +3)
                    (-1..=2).flat_map(|r| (-2..=3).map(move |c| (r, c))),
                )
                .chain(
                    // rect (+3, -1) ..= (+3, +2)
                    (-1..=2).map(|c| (3, c)),
                );

            for (r, c) in treasure_rooms {
                affected_coords
                    .clone()
                    .map(|(dr, dc)| (r + dr, c + dc))
                    .filter_map(|(r, c)| {
                        (self.is_in_bounds(r, c)).then_some((r as usize, c as usize))
                    })
                    .for_each(|(r, c)| {
                        check[r][c] = false;
                    });
            }

            check
        };

        for i in 0..BOARD_SIZE - 1 {
            for j in 0..BOARD_SIZE - 1 {
                if !coords_to_check[i][j] {
                    continue;
                }
                let is_empty_2x2 = [(i, j), (i, j + 1), (i + 1, j), (i + 1, j + 1)]
                    .into_iter()
                    .all(|(i, j)| matches!(self.cells[i][j], Cell::Empty));
                if is_empty_2x2 {
                    return Err(E::CorridorsTooWide(i as Index, j as Index));
                }
            }
        }

        let first_empty_cell = (0..BOARD_SIZE)
            .flat_map(|r| (0..BOARD_SIZE).map(move |c| (r, c)))
            .find(|&(r, c)| matches!(self.cells[r][c], Cell::Empty))
            .map(|(r, c)| (r as Index, c as Index));
        let mut to_check: Vec<(Index, Index)> = first_empty_cell.into_iter().collect();
        let mut seen = [[false; BOARD_SIZE]; BOARD_SIZE];
        let mut connected_cells: u32 = 0;

        while let Some((r, c)) = to_check.pop() {
            let seen_this = &mut seen[r as usize][c as usize];
            if *seen_this {
                continue;
            }
            *seen_this = true;
            connected_cells += 1;
            let neighbors = [(r - 1, c), (r, c - 1), (r, c + 1), (r + 1, c)];
            to_check.extend(
                // TODO this kinda ugly... `seen` is unelegant & maybe a footgun
                neighbors
                    .into_iter()
                    .filter(|&(r, c)| !matches!(self.at(r, c), Cell::Wall)),
            )
        }

        if first_empty_cell.is_none() {
            return Ok(()); // unlikely, but who knows?
        }

        let total_empty = self
            .cells
            .iter()
            .flatten()
            .filter(|&c| !matches!(c, Cell::Wall))
            .count() as u32;

        // All empty cells are connected
        if connected_cells != total_empty {
            return Err(E::UnconnectedCorridors);
        }

        Ok(())
    }

    // Accepts out-of-bounds coordinates, and assumes there are walls everywhere outside the board.
    fn at(&self, r: Index, c: Index) -> Cell {
        if self.is_in_bounds(r, c) {
            self.cells[r as usize][c as usize]
        } else {
            Cell::Wall
        }
    }

    fn is_in_bounds(&self, r: Index, c: Index) -> bool {
        (0..BOARD_SIZE as Index).contains(&r) && (0..BOARD_SIZE as Index).contains(&c)
    }

    fn is_dead_end(&self, r: Index, c: Index) -> bool {
        if matches!(self.at(r, c), Cell::Unknown | Cell::Wall) {
            return false;
        }
        let surrounding_wall_count = [(r - 1, c), (r, c - 1), (r, c + 1), (r + 1, c)]
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Wall))
            .count();

        surrounding_wall_count == 3
    }

    fn maybe_dead_end(&self, r: Index, c: Index) -> bool {
        let surrounding_cells = [(r - 1, c), (r, c - 1), (r, c + 1), (r + 1, c)];
        let walls = surrounding_cells
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Wall))
            .count();
        let air = surrounding_cells
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Empty))
            .count();

        walls <= 3 && air <= 1
    }

    fn maybe_treasure_room(&self, r: Index, c: Index) -> bool {
        let inside_coords = [
            (r, c),
            (r, c + 1),
            (r, c + 2),
            (r + 1, c),
            (r + 1, c + 1),
            (r + 1, c + 2),
            (r + 2, c),
            (r + 2, c + 1),
            (r + 2, c + 2),
        ];
        let mut chest_seen = false;
        for (r, c) in inside_coords {
            match self.at(r, c) {
                Cell::Chest => {
                    if chest_seen {
                        return false;
                    };
                    chest_seen = true;
                }
                Cell::Empty | Cell::Unknown => {}
                _ => {
                    return false;
                }
            }
        }
        let outside_coords = [
            // top
            (r - 1, c),
            (r - 1, c + 1),
            (r - 1, c + 2),
            //left-right
            (r, c - 1),
            (r, c + 3),
            (r + 1, c - 1),
            (r + 1, c + 3),
            (r + 2, c - 1),
            (r + 2, c + 3),
            // bottom
            (r + 3, c),
            (r + 3, c + 1),
            (r + 3, c + 2),
        ];
        let wall_count = outside_coords
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Wall))
            .count();
        let unknown_count = outside_coords
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Unknown))
            .count();

        (wall_count..=wall_count + unknown_count).contains(&(outside_coords.len() - 1))
    }

    fn is_treasure_room(&self, r: Index, c: Index) -> bool {
        let inside_coords = [
            (r, c),
            (r, c + 1),
            (r, c + 2),
            (r + 1, c),
            (r + 1, c + 1),
            (r + 1, c + 2),
            (r + 2, c),
            (r + 2, c + 1),
            (r + 2, c + 2),
        ];
        let mut chest_seen = false;
        for (r, c) in inside_coords {
            match self.at(r, c) {
                Cell::Chest => {
                    if chest_seen {
                        return false;
                    };
                    chest_seen = true;
                }
                Cell::Empty => {}
                _ => {
                    return false;
                }
            }
        }
        let outside_coords = [
            // top
            (r - 1, c),
            (r - 1, c + 1),
            (r - 1, c + 2),
            //left-right
            (r, c - 1),
            (r, c + 3),
            (r + 1, c - 1),
            (r + 1, c + 3),
            (r + 2, c - 1),
            (r + 2, c + 3),
            // bottom
            (r + 3, c),
            (r + 3, c + 1),
            (r + 3, c + 2),
        ];
        let wall_count = outside_coords
            .into_iter()
            .filter(|&(r, c)| matches!(self.at(r, c), Cell::Wall))
            .count();

        wall_count == outside_coords.len() - 1
    }

    fn solve(&mut self) -> Result<(), Unsolvable> {
        let first_unknown = (0..BOARD_SIZE)
            .flat_map(|r| (0..BOARD_SIZE).map(move |c| (r, c)))
            .find(|&(r, c)| matches!(self.cells[r][c], Cell::Unknown));
        if let Some((r, c)) = first_unknown {
            self.cells[r][c] = Cell::Wall;
            if self.maybe_solvable().is_ok() && self.solve().is_ok() {
                return Ok(());
            }
            self.cells[r][c] = Cell::Empty;
            if self.maybe_solvable().is_ok() && self.solve().is_ok() {
                return Ok(());
            }
            self.cells[r][c] = Cell::Unknown;

            Err(Unsolvable)
        } else {
            self.check_solved().or(Err(Unsolvable))
        }
    }

    fn maybe_solvable(&self) -> Result<(), BoardError> {
        self.rows_acceptable()
            .map_err(|i| BoardError::WrongRowCount(i))?;
        self.cols_acceptable()
            .map_err(|i| BoardError::WrongColumnCount(i))?;

        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                let is_monster = matches!(self.cells[i][j], Cell::Monster);
                let (r, c) = (i as Index, j as Index);
                let maybe_dead_end = self.maybe_dead_end(r, c);
                let is_dead_end = self.is_dead_end(r, c);
                if is_monster && !maybe_dead_end {
                    return Err(BoardError::MonsterNotInDeadEnd(r, c));
                }
                if !is_monster && is_dead_end {
                    return Err(BoardError::DeadEndWithNoMontster(r, c));
                }

                if let Cell::Chest = self.cells[i][j] {
                    let treasure_room_candidates = [
                        (r - 2, c - 2),
                        (r - 2, c - 1),
                        (r - 2, c),
                        (r - 1, c - 2),
                        (r - 1, c - 1),
                        (r - 1, c),
                        (r, c - 2),
                        (r, c - 1),
                        (r, c),
                    ];
                    let maybe_room = treasure_room_candidates
                        .into_iter()
                        .find(|&(r, c)| self.maybe_treasure_room(r, c));
                    if maybe_room.is_none() {
                        return Err(BoardError::NoTreasureRoomForChest(r, c));
                    }
                }
            }
        }

        Ok(())
    }
}

fn main() {
    let mut puzzle_5_8 = Board::from_string(
        " 35344253\n\
         4M   M M \n\
         4        \n\
         2M       \n\
         4       M\n\
         6M       \n\
         2       M\n\
         3        \n\
         4 M   M M",
    )
    .unwrap();
    println!("{:}", puzzle_5_8);
    puzzle_5_8.solve().expect("unsolvable");
    println!("{:}", puzzle_5_8);

    let good1 = Board::from_string(
        " 88888888\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let good2 = Board::from_string(
        " 88878888\n\
         8########\n\
         8########\n\
         7###.####\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let good3 = Board::from_string(
        " 87775658\n\
         8########\n\
         5####...#\n\
         3#M...#.#\n\
         5####...#\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let good4 = Board::from_string(
        " 35255888\n\
         8########\n\
         5##...###\n\
         5##..C###\n\
         3.....###\n\
         7.#######\n\
         5...#####\n\
         6.#.#####\n\
         5...#####",
    )
    .unwrap();

    println!("good1.check_solved() = {:?}", good1.check_solved());
    println!("good2.check_solved() = {:?}", good2.check_solved());
    println!("good3.check_solved() = {:?}", good3.check_solved());
    println!("good4.check_solved() = {:?}", good4.check_solved());

    let bad1 = Board::from_string(
        " 88888188\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let bad2 = Board::from_string(
        " 88877688\n\
         8########\n\
         7#####.##\n\
         5###...##\n\
         8########\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let bad3 = Board::from_string(
        " 88885658\n\
         8########\n\
         5####...#\n\
         6####.#.#\n\
         5####M..#\n\
         8########\n\
         8########\n\
         8########\n\
         8########",
    )
    .unwrap();
    let bad4 = Board::from_string(
        " 56443888\n\
         8########\n\
         5##...###\n\
         5##..C###\n\
         3.....###\n\
         6.###.###\n\
         3.....###\n\
         8########\n\
         8########",
    )
    .unwrap();
    let bad5 = Board::from_string(
        " 84645658\n\
         8########\n\
         5#...####\n\
         6#.#.####\n\
         6#.#.####\n\
         5#...####\n\
         5####...#\n\
         6####.#.#\n\
         5####...#",
    )
    .unwrap();
    let bad6 = Board::from_string(
        " 88882458\n\
         8########\n\
         8########\n\
         6####..##\n\
         6####..##\n\
         7####.###\n\
         5####...#\n\
         6####.#.#\n\
         5####...#",
    )
    .unwrap();

    println!("bad1.check_solved() = {:?}", bad1.check_solved());
    println!("bad2.check_solved() = {:?}", bad2.check_solved());
    println!("bad3.check_solved() = {:?}", bad3.check_solved());
    println!("bad4.check_solved() = {:?}", bad4.check_solved());
    println!("bad5.check_solved() = {:?}", bad5.check_solved());
    println!("bad6.check_solved() = {:?}", bad6.check_solved());
}
