# Stack technologiczny

**Rust** – język programowania. Służy do napisania programu pobierającego recenzje przez Steam Web API. Program zapisuje recenzje do plików JSON z pominięciem pustych, a w osobnym kroku tworzy oczyszczony plik bez duplikatów i z wyciętymi blokami ASCII art.

**Steam Web API** – interfejs programistyczny Steama udostępniający dane o grach. Jest źródłem danych: `IUserReviewsService/GetAppReviews` dostarcza recenzje (tekst, datę, rekomendację „polecam / nie polecam” i czas gry autora). Daty aktualizacji planujemy pozyskać z oficjalnych ogłoszeń udostępnianych przez `ISteamNews/GetNewsForApp`.

**Python** – język programowania. Służy do analizy danych, budowy bazy oraz aplikacji.

**SQLite** – baza danych w jednym pliku, bez osobnego serwera, obsługiwana przez wbudowany w Pythona moduł `sqlite3`. Przechowuje gry, recenzje, daty patchy, wyniki modeli i gotowe zestawienia; aplikacja tylko z niej czyta.

**pandas** – biblioteka do pracy na tabelach danych. Służy do wczytywania danych z bazy, liczenia zestawień w wybranych okresach, np. odsetka pozytywnych recenzji według rekomendacji autorów i według RoBERTy, oraz wybierania recenzji danej gry z danego okresu do analizy słów.

**NLTK** – biblioteka do przetwarzania tekstu. Służy do wstępnego przygotowania recenzji, m.in. podziału na zdania i lematyzacji, czyli sprowadzania słów do formy podstawowej.

**RoBERTa (transformers + PyTorch)** – model językowy uruchamiany za pomocą bibliotek transformers i PyTorch. Służy do określania sentymentu recenzji.

**scikit-learn (CountVectorizer)** – biblioteka do uczenia maszynowego, z której wykorzystujemy narzędzie CountVectorizer do zliczania słów i par słów w recenzjach. Pozwala sprawdzić, które słowa pojawiały się częściej w okresach zmiany sentymentu.

**Plotly** – biblioteka do interaktywnych wykresów z przybliżaniem i podpowiedziami po najechaniu kursorem. Służy do wizualizacji wyników analizy, przede wszystkim odsetka pozytywnych recenzji w czasie na tle dat patchy, a także innych danych, np. czasu gry recenzentów.

**Streamlit** – biblioteka, która zamienia skrypt w Pythonie w aplikację otwieraną lokalnie w przeglądarce. Służy do zbudowania interaktywnego panelu, w którym użytkownik przegląda wyniki analizy dla wybranych gier.
