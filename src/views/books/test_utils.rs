use std::{collections::HashSet, env::temp_dir};

use rand::{distributions::Alphanumeric, Rng};

use super::RootBookDir;

pub const TXT: &str = "A lei tenho daquele, a cujo império
Obedece o visíbil e ínvisíbil
Aquele que criou todo o Hemisfério,
Tudo o que sente, o todo o insensíbil;
Que padeceu desonra e vitupério,
Sofrendo morte injusta e insofríbil,
E que do Céu à Terra, enfim desceu,
Por subir os mortais da Terra ao Céu.

Deste Deus-Homem, alto e infinito,
Os livros, que tu pedes não trazia,
Que bem posso escusar trazer escrito
Em papel o que na alma andar devia.
Se as armas queres ver, como tens dito,
Cumprido esse desejo te seria;
Como amigo as verás; porque eu me obrigo,
Que nunca as queiras ver como inimigo.

Isto dizendo, manda os diligentes
Ministros amostrar as armaduras:
Vêm arneses, e peitos reluzentes,
Malhas finas, e lâminas seguras,
Escudos de pinturas diferentes,
Pelouros, espingardas de aço puras,
Arcos, e sagitíferas aljavas,
Partazanas agudas, chuças bravas:";
pub fn s(v: Vec<&str>) -> HashSet<String> {
    v.into_iter().map(|v| v.to_string()).collect()
}
pub fn create_book_dir() -> RootBookDir {
    let random_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let book_dir = temp_dir()
        .to_path_buf()
        .join("bookrab-test".to_string() + &random_name);
    let root = RootBookDir::new(book_dir);
    root.create().expect("couldnt create root dir");
    root
}
pub fn root_for_tag_tests() -> RootBookDir {
    let book_dir = temp_dir().to_path_buf().join("tag_testing_bookrab");
    let root = RootBookDir::new(book_dir);
    if root.path.exists() {
        return root;
    }
    root.create().expect("couldnt create root dir");
    root.upload("1", "", s(vec!["a", "b", "c", "d"]))
        .unwrap()
        .upload("2", "", s(vec!["a", "b", "c"]))
        .unwrap()
        .upload("3", "", s(vec!["a", "b"]))
        .unwrap()
        .upload("4", "", s(vec!["a"]))
        .unwrap();
    root
}
pub fn basic_metadata() -> HashSet<String> {
    vec!["Camões".to_string(), "Literatura Portuguesa".to_string()]
        .into_iter()
        .collect()
}
