// Inline and display math in New Computer Modern Math, numbered equations.
#set page(width: 12cm, height: auto, margin: 1cm)
#set text(font: "Libertinus Serif", size: 10pt)
#set math.equation(numbering: "(1)")

Let $f(x) = sum_(k=0)^n a_k x^k$ be a polynomial with $a_n != 0$. Then

$ integral_0^1 f(x) dif x = sum_(k=0)^n a_k / (k+1) $ <int>

and @int holds for every $n in NN$. A matrix:

$ mat(1, 2; 3, 4) vec(x, y) = sqrt(x^2 + y^2) $
