# FastFind

_Busca archivos de manera eficiente y rápida. Elige el directorio donde deseas buscar. Usa comodines para ajustar tu búsqueda. También puedes moverte por la lista usando las teclas de dirección `⬆` y `⬇`. Además, puedes abrir estos archivos haciendo `Enter`._

## Comodines:

- `*` : 0 o más caracteres
- `?` : 1 caracter ; `??` : 2 caracteres ; `???` : 3 caracteres ; etc.

## Capturas:
<details>
  <summary>Desplegar para ver las capturas</summary>
  <br>
  
  - Menú principal:
    
  ![Menu](img/1.webp)
  > Aquí es donde escribes el directorio donde deseas realizar la búsqueda.
<br>

  - Buscando archivos con `1` caracter por nombre con extensión `.rs`:

  ![Archivos con 1 caracter](img/2.webp)
<br>

  - Buscando archivos con `3` caracter por nombre con extensión `.js`:

  ![Archivos con 3 caracter](img/3.webp)
<br>

  - Buscando todos los archivos que acaben con la extensión `.txt`:

  ![Archivos acabados en .txt](img/4.webp)
<br>

  - Buscando archivos con nombre específico:

  ![Archivos con nombre específico 1](img/5.webp)
  ![Archivos con nombre específico 2](img/6.webp)
<br>

  - Mostrar todos los archivos del directorio:

  ![Todos los archivos](img/7.webp)
</details>

> [!NOTE]
> **Recomendación:**
> <br>_Añadir FastFind como comando en tu sistema y un alias para un uso rápido._
> <br><br>
> **Ejemplo:**
> <br>
> `sudo ln -s ~/fastfind/target/release/fastfind ~/.local/bin/fastfind`
> <br><br>
> **Dentro de `~/.bashrc`:**
> <br>
> `alias ff='fastfind'`
