use std::{collections::HashSet, env::temp_dir};

use diesel::r2d2::ConnectionManager;
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    config::{ensure_config_works, BookrabConfig},
    database::{PgPool, PgPooledConnection},
};

use super::RootBookDir;

pub const LUSIADAS1: &str = "A lei tenho daquele, a cujo império
Obedece o visíbil e ínvisíbil
Aquele que criou todo o Hemisfério,
Tudo o que sente, e todo o insensíbil;
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
pub const LUSIADAS2: &str = "As armas e os barões assinalados,
Que da ocidental praia Lusitana,
Por mares nunca de antes navegados,
Passaram ainda além da Taprobana,
Em perigos e guerras esforçados,
Mais do que prometia a força humana,
E entre gente remota edificaram
Novo Reino, que tanto sublimaram;

E também as memórias gloriosas
Daqueles Reis, que foram dilatando
A Fé, o Império, e as terras viciosas
De África e de Ásia andaram devastando;
E aqueles, que por obras valerosas
Se vão da lei da morte libertando;
Cantando espalharei por toda parte,
Se a tanto me ajudar o engenho e arte.

Cessem do sábio Grego e do Troiano
As navegações grandes que fizeram;
Cale-se de Alexandro e de Trajano
A fama das vitórias que tiveram;
Que eu canto o peito ilustre Lusitano,
A quem Neptuno e Marte obedeceram:
Cesse tudo o que a Musa antiga canta,
Que outro valor mais alto se alevanta.";
pub const LUSIADAS3: &str = "Era tão grande o peso do madeiro
Que, só pera abalar-se, nada abasta;
Mas o núncio de Cristo verdadeiro
Menos trabalho em tal negócio gasta:
Ata o cordão que traz, por derradeiro,
No tronco, e fàcilmente o leva e arrasta
Pera onde faça um sumptuoso templo
Que ficasse aos futuros por exemplo.

Sabia bem que se com fé formada
Mandar a um monte surdo que se mova,
Que obedecerá logo à voz sagrada,
Que assi lho ensinou Cristo, e ele o prova.
A gente ficou disto alvoraçada;
Os Brâmenes o têm por cousa nova;
Vendo os milagres, vendo a santidade,
Hão medo de perder autoridade.

São estes sacerdotes dos Gentios
Em quem mais penetrado tinha enveja;
Buscam maneiras mil, buscam desvios,
Com que Tomé não se ouça, ou morto seja.
O principal, que ao peito traz os fios,
Um caso horrendo faz, que o mundo veja
Que inimiga não há, tão dura e fera,
Como a virtude falsa, da sincera.";
pub const LUSIADAS4: &str = "Um filho próprio mata, e logo acusa
De homicídio Tomé, que era inocente;
Dá falsas testemunhas, como se usa;
Condenaram-no a morte brevemente.
O Santo, que não vê milhor escusa
Que apelar pera o Padre omnipotente,
Quer, diante do Rei e dos senhores,
Que se faça um milagre dos maiores.

O corpo morto manda ser trazido,
Que res[s]ucite e seja perguntado
Quem foi seu matador, e será crido
Por testemunho, o seu, mais aprovado.
Viram todos o moço vivo, erguido,
Em nome de Jesu crucificado:
Dá graças a Tomé, que lhe deu vida,
E descobre seu pai ser homicida.

Este milagre fez tamanho espanto
Que o Rei se banha logo na água santa,
E muitos após ele; um beija o manto,
Outro louvor do Deus de Tomé canta.
Os Brâmenes se encheram de ódio tanto,
Com seu veneno os morde enveja tanta,
Que, persuadindo a isso o povo rudo,
Determinam matá-lo, em fim de tudo.";

lazy_static! {
    pub static ref DBCONNECTION: PgPool = create_test_connection_pool();
}

pub fn s(v: Vec<&str>) -> HashSet<String> {
    v.into_iter().map(|v| v.to_string()).collect()
}

pub fn create_book_dir(connection: &mut PgPooledConnection) -> RootBookDir<'_> {
    let random_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect();

    let temp = temp_dir().to_path_buf();
    let book_dir = temp.join("bookrab-test-".to_string() + &random_name);
    RootBookDir::new(
        ensure_config_works(&BookrabConfig {
            book_path: book_dir,
            database_url: String::from("postgres://bookrab:bookStrongPass@localhost/bookrab_db"),
        })
        .clone(),
        connection,
    )
}

/// Generates a root folder for books.
/// The following books will be available:
/// title: "1", tags:  ["a", "b", "c", "d"]
/// title: "2", tags:  ["a", "b", "c"]
/// title: "3", tags:  ["a", "b"]
/// title: "4", tags:  ["a"]
pub fn root_for_tag_tests(connection: &mut PgPooledConnection) -> RootBookDir {
    let temp = temp_dir().to_path_buf();
    let book_dir = temp.join("tag_testing_bookrab");

    let config = BookrabConfig {
        book_path: book_dir,
        database_url: String::from("postgres://bookrab:bookStrongPass@localhost/bookrab_db"),
    };
    if config.book_path.exists() {
        return RootBookDir::new(ensure_config_works(&config).clone(), connection);
    }
    let root = RootBookDir::new(ensure_config_works(&config).clone(), connection);
    root.upload("1", LUSIADAS1, s(vec!["a", "b", "c", "d"]))
        .unwrap()
        .upload("2", LUSIADAS2, s(vec!["a", "b", "c"]))
        .unwrap()
        .upload("3", LUSIADAS3, s(vec!["a", "b"]))
        .unwrap()
        .upload("4", LUSIADAS4, s(vec!["a"]))
        .unwrap();
    root
}

pub fn basic_metadata() -> HashSet<String> {
    vec!["Camões".to_string(), "Literatura Portuguesa".to_string()]
        .into_iter()
        .collect()
}

pub fn create_test_connection_pool() -> PgPool {
    let connection_string = "postgres://bookrab:bookStrongPass@localhost/bookrab_db";
    PgPool::builder()
        .max_size(8)
        .build(ConnectionManager::new(connection_string))
        .expect("failed to create db connection_pool")
}
