CREATE TABLE search_history (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  pattern VARCHAR NOT NULL,
  date timestamp NOT NULL DEFAULT NOW()
);

CREATE TABLE search_results (
  id SERIAL PRIMARY KEY,
  search_history_id INT REFERENCES search_history(id) NOT NULL,
  result TEXT NOT NULL
);
